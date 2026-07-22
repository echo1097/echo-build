use agent_client_protocol as acp;

pub(crate) type SharedAuthMethodId = std::sync::Arc<arc_swap::ArcSwapOption<acp::AuthMethodId>>;

pub(crate) fn new_shared_auth_method_id(initial: Option<acp::AuthMethodId>) -> SharedAuthMethodId {
    std::sync::Arc::new(arc_swap::ArcSwapOption::new(
        initial.map(std::sync::Arc::new),
    ))
}

pub const OPENROUTER_API_KEY_METHOD_ID: &str = "openrouter.api_key";
pub const LEGACY_XAI_API_KEY_METHOD_ID: &str = "xai.api_key";

#[cfg(test)]
pub const XAI_API_KEY_METHOD_ID: &str = OPENROUTER_API_KEY_METHOD_ID;
#[cfg(test)]
pub const XAI_API_KEY_ENV_VAR: &str = "XAI_API_KEY";
#[cfg(test)]
pub const LEGACY_XAI_API_KEY_ENV_VAR: &str = "GROK_CODE_XAI_API_KEY";
pub(crate) const CACHED_TOKEN_AUTH_METHOD_ID: &str = "cached_token";
pub(crate) const GROK_COM_METHOD_ID: &str = "grok.com";
pub(crate) const OIDC_METHOD_ID: &str = "oidc";

pub fn openrouter_api_key_auth_method() -> acp::AuthMethod {
    acp::AuthMethod::Agent(
        acp::AuthMethodAgent::new(
            acp::AuthMethodId::new(OPENROUTER_API_KEY_METHOD_ID),
            "OpenRouter API key".to_string(),
        )
        .description(Some(
            "Stored securely in the operating system credential store".to_string(),
        )),
    )
}

pub fn build_auth_methods() -> Vec<acp::AuthMethod> {
    vec![openrouter_api_key_auth_method()]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethodKind {
    OpenRouterApiKey,
    Unknown,
}

impl AuthMethodKind {
    pub fn from_id(id: &acp::AuthMethodId) -> Self {
        match id.0.as_ref() {
            OPENROUTER_API_KEY_METHOD_ID | LEGACY_XAI_API_KEY_METHOD_ID => Self::OpenRouterApiKey,
            _ => Self::Unknown,
        }
    }

    pub fn is_api_key(self) -> bool {
        matches!(self, Self::OpenRouterApiKey)
    }

    pub fn needs_interactive_login(self) -> bool {
        false
    }
}

pub fn is_session_based_method(_method_id: &acp::AuthMethodId) -> bool {
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelByok {
    Byok,
    NotByok,
    Unknown,
}

impl ModelByok {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Byok => "byok",
            Self::NotByok => "not_byok",
            Self::Unknown => "unknown",
        }
    }
}

pub fn session_token_auth_gate(
    _is_session_based_method: bool,
    _model_byok: ModelByok,
    _endpoint_is_first_party: bool,
) -> bool {
    false
}

pub const AUTH_ERROR_API_KEY: &str =
    "Authentication failed. Add an OpenRouter API key to the operating system credential store.";

pub(crate) const PREFERRED_API_KEY_UNAVAILABLE: &str = AUTH_ERROR_API_KEY;
pub(crate) const PREFERRED_OIDC_UNAVAILABLE: &str = AUTH_ERROR_API_KEY;

pub(crate) fn method_id_after_cached_token_unavailable(
    _has_external_api_key: bool,
    _preferred_method: Option<crate::auth::PreferredAuthMethod>,
) -> Option<&'static str> {
    Some(OPENROUTER_API_KEY_METHOD_ID)
}

pub(crate) fn should_advertise_xai_api_key<'a, I>(_disable_api_key_auth: bool, _models: I) -> bool
where
    I: IntoIterator<Item = &'a crate::agent::config::ModelEntry>,
{
    crate::auth::cached_api_key().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_only_openrouter_api_key() {
        let methods = build_auth_methods();

        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].id().0.as_ref(), OPENROUTER_API_KEY_METHOD_ID);
        assert_eq!(methods[0].name(), "OpenRouter API key");
    }

    #[test]
    fn accepts_canonical_and_legacy_openrouter_method_ids() {
        let canonical = acp::AuthMethodId::new(OPENROUTER_API_KEY_METHOD_ID);
        let legacy = acp::AuthMethodId::new(LEGACY_XAI_API_KEY_METHOD_ID);
        let unsupported = acp::AuthMethodId::new("grok.com");

        assert!(AuthMethodKind::from_id(&canonical).is_api_key());
        assert!(AuthMethodKind::from_id(&legacy).is_api_key());
        assert_eq!(
            AuthMethodKind::from_id(&unsupported),
            AuthMethodKind::Unknown
        );
    }

    #[test]
    fn no_auth_method_can_start_interactive_login() {
        let unsupported = acp::AuthMethodId::new("oidc");

        assert!(!AuthMethodKind::from_id(&unsupported).needs_interactive_login());
        assert!(!is_session_based_method(&unsupported));
    }
}
