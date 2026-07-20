//! OpenRouter credential handlers plus legacy local ACP aliases.
//!
//! These methods let the client read/write the API key via the agent and
//! drive the OAuth login flow. The agent is the single source of truth for
//! `auth.json`.

use agent_client_protocol as acp;
use serde::Serialize;

use super::{ExtResult, parse_params, to_raw_response};
use crate::agent::MvpAgent;
use crate::session::ExtMethodResult;

#[tracing::instrument(skip_all, fields(method = %args.method))]
pub async fn handle(agent: &MvpAgent, args: &acp::ExtRequest) -> ExtResult {
    match args.method.as_ref() {
        "x.ai/auth/getBearerToken" => handle_get_api_key(),
        "echo.openrouter/getApiKeyStatus" | "x.ai/getApiKey" => handle_get_api_key(),
        "echo.openrouter/setApiKey" | "x.ai/setApiKey" => handle_set_api_key(agent, args).await,
        "x.ai/auth/logout" => handle_logout(agent, args).await,
        "x.ai/auth/info" => handle_info(agent),
        _ => Err(acp::Error::method_not_found()),
    }
}

fn handle_get_api_key() -> ExtResult {
    let configured = crate::auth::cached_api_key().is_some();
    ExtMethodResult::success(serde_json::json!({ "configured": configured, "key": null }))
        .to_ext_response()
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))
}

async fn handle_set_api_key(agent: &MvpAgent, args: &acp::ExtRequest) -> ExtResult {
    let params: serde_json::Value = parse_params(args)?;
    let key = params.get("key").and_then(|v| v.as_str());
    let grok_home = crate::util::grok_home::grok_home();
    if let Some(k) = key {
        if k.is_empty() {
            agent.sampling_config.borrow_mut().api_key = None;
            crate::auth::delete_api_key_and_legacy(&grok_home)
                .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;
        } else {
            agent.sampling_config.borrow_mut().api_key = None;
            crate::auth::save_api_key(&grok_home, k)
                .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;
            agent.sampling_config.borrow_mut().api_key = Some(k.to_owned());
        }
    } else {
        agent.sampling_config.borrow_mut().api_key = None;
        crate::auth::delete_api_key_and_legacy(&grok_home)
            .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;
    }
    agent.models_manager.on_auth_changed().await;
    ExtMethodResult::success(serde_json::json!({ "ok": true }))
        .to_ext_response()
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))
}

async fn handle_logout(agent: &MvpAgent, args: &acp::ExtRequest) -> ExtResult {
    let _ = args;
    agent.interactive_auth.cancel();
    agent.auth_manager.clear_in_memory();
    agent.sampling_config.borrow_mut().api_key = None;
    crate::auth::delete_api_key_and_legacy(&crate::util::grok_home::grok_home())
        .map_err(|error| acp::Error::internal_error().data(error.to_string()))?;
    let api_key_still_set = crate::agent::auth_method::has_xai_api_key_env();
    // `auth.lifecycle` (not `auth`) avoids colliding with the pre-existing
    // per-request `AuthManager::auth()` `#[instrument]` span.
    tracing::info_span!("auth.lifecycle", action = "logout", success = true).in_scope(|| {});

    agent.models_manager.on_auth_changed().await;

    to_raw_response(&serde_json::json!({
        "ok": true,
        "was_logged_in": true,
        "email": null,
        "api_key_still_set": api_key_still_set,
    }))
}

/// Returns current auth method ID, user profile fields, and team/principal
/// metadata.
fn handle_info(agent: &MvpAgent) -> ExtResult {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AuthInfoResponse {
        method_id: Option<String>,
        email: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        /// `grok-asset://` URL resolved by the Electron protocol handler,
        /// or a full `http(s)://` URL passed through unchanged.
        profile_image_url: Option<String>,
        team_id: Option<String>,
        team_name: Option<String>,
        team_role: Option<String>,
        organization_id: Option<String>,
        organization_name: Option<String>,
        organization_role: Option<String>,
        principal_type: Option<String>,
        principal_id: Option<String>,
        user_blocked_reason: Option<String>,
        team_blocked_reasons: Vec<String>,
        coding_data_retention_opt_out: bool,
    }

    let method_id = agent
        .auth_method_id
        .load()
        .as_ref()
        .map(|m| m.0.to_string());
    to_raw_response(&AuthInfoResponse {
        method_id,
        email: None,
        first_name: None,
        last_name: None,
        profile_image_url: None,
        team_id: None,
        team_name: None,
        team_role: None,
        organization_id: None,
        organization_name: None,
        organization_role: None,
        principal_type: Some("openrouter".to_string()),
        principal_id: None,
        user_blocked_reason: None,
        team_blocked_reasons: Vec::new(),
        coding_data_retention_opt_out: true,
    })
}
