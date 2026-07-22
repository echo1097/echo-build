//! OpenRouter credential handlers behind the ACP compatibility adapter.

use super::{ExtResult, parse_params, to_raw_response};
use crate::agent::MvpAgent;
use crate::session::ExtMethodResult;
use agent_client_protocol as acp;

#[tracing::instrument(skip_all, fields(method = %args.method))]
pub async fn handle(agent: &MvpAgent, args: &acp::ExtRequest) -> ExtResult {
    match args.method.as_ref() {
        "x.ai/auth/getBearerToken" => handle_get_api_key(),
        "x.ai/getApiKey" => handle_get_api_key(),
        "x.ai/setApiKey" => handle_set_api_key(agent, args).await,
        "x.ai/auth/logout" => handle_logout(agent, args).await,
        "x.ai/auth/info" => handle_info(agent),
        _ => Err(acp::Error::method_not_found()),
    }
}

fn handle_get_api_key() -> ExtResult {
    let configured = crate::auth::cached_api_key().is_some();
    ExtMethodResult::success(serde_json::json!({ "configured": configured }))
        .to_ext_response()
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))
}

async fn handle_set_api_key(agent: &MvpAgent, args: &acp::ExtRequest) -> ExtResult {
    let params: serde_json::Value = parse_params(args)?;
    let key = params
        .get("key")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| acp::Error::invalid_params().data("OpenRouter API key cannot be empty."))?;
    let grok_home = crate::util::grok_home::grok_home();
    crate::auth::save_api_key(&grok_home, key)
        .map_err(|error| acp::Error::internal_error().data(error.to_string()))?;
    agent.sampling_config.borrow_mut().api_key = Some(key.to_owned());
    agent.models_manager.on_auth_changed().await;
    ExtMethodResult::success(serde_json::json!({ "configured": true }))
        .to_ext_response()
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))
}

async fn handle_logout(agent: &MvpAgent, args: &acp::ExtRequest) -> ExtResult {
    let _ = args;
    agent.interactive_auth.cancel();
    crate::auth::delete_api_key_and_legacy(&crate::util::grok_home::grok_home())
        .map_err(|error| acp::Error::internal_error().data(error.to_string()))?;
    agent.auth_manager.clear_in_memory();
    agent.sampling_config.borrow_mut().api_key = None;
    agent.auth_method_id.store(None);
    // `auth.lifecycle` (not `auth`) avoids colliding with the pre-existing
    // per-request `AuthManager::auth()` `#[instrument]` span.
    tracing::info_span!("auth.lifecycle", action = "logout", success = true).in_scope(|| {});

    agent.models_manager.on_auth_changed().await;

    to_raw_response(&serde_json::json!({ "configured": false }))
}

fn handle_info(agent: &MvpAgent) -> ExtResult {
    let _ = agent;
    to_raw_response(&serde_json::json!({
        "configured": crate::auth::cached_api_key().is_some()
    }))
}
