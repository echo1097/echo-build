//! Echo Build ACP namespace compatibility at the public protocol boundary.

pub const CANONICAL_NAMESPACE: &str = "echo.build";
pub const AUTH_NAMESPACE: &str = "echo.openrouter";
pub const LEGACY_NAMESPACE: &str = "x.ai";
pub const LEGACY_ALIAS_REMOVAL_VERSION: &str = "0.3.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodRoute {
    pub internal_method: String,
    pub legacy_alias: bool,
}

pub fn route_method(method: &str) -> MethodRoute {
    let internal_method = match method {
        "echo.openrouter/getApiKeyStatus" => "x.ai/getApiKey".to_owned(),
        "echo.openrouter/status" => "x.ai/getApiKey".to_owned(),
        "echo.openrouter/setApiKey" => "x.ai/setApiKey".to_owned(),
        "echo.openrouter/setKey" => "x.ai/setApiKey".to_owned(),
        "echo.openrouter/clearKey" => "x.ai/auth/logout".to_owned(),
        "echo.openrouter/logout" => "x.ai/auth/logout".to_owned(),
        "echo.openrouter/info" => "x.ai/auth/info".to_owned(),
        _ if method.starts_with("echo.build/") => {
            format!("x.ai/{}", method.trim_start_matches("echo.build/"))
        }
        _ => method.to_owned(),
    };

    MethodRoute {
        internal_method,
        legacy_alias: method.starts_with("x.ai/"),
    }
}

pub fn capabilities() -> serde_json::Value {
    serde_json::json!({
        "product": "Echo Build",
        "namespace": CANONICAL_NAMESPACE,
        "authNamespace": AUTH_NAMESPACE,
        "legacyNamespace": LEGACY_NAMESPACE,
        "legacyAliasRemovalVersion": LEGACY_ALIAS_REMOVAL_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_runtime_method_routes_to_inherited_handler_name() {
        let route = route_method("echo.build/session/list");

        assert_eq!(route.internal_method, "x.ai/session/list");
        assert!(!route.legacy_alias);
    }

    #[test]
    fn legacy_method_is_marked_deprecated_without_params() {
        let route = route_method("x.ai/session/list");

        assert_eq!(route.internal_method, "x.ai/session/list");
        assert!(route.legacy_alias);
    }

    #[test]
    fn openrouter_auth_has_a_separate_namespace() {
        let expected = [
            ("echo.openrouter/status", "x.ai/getApiKey"),
            ("echo.openrouter/setKey", "x.ai/setApiKey"),
            ("echo.openrouter/clearKey", "x.ai/auth/logout"),
            ("echo.openrouter/logout", "x.ai/auth/logout"),
            ("echo.openrouter/info", "x.ai/auth/info"),
        ];

        for (canonical, internal) in expected {
            let route = route_method(canonical);
            assert_eq!(route.internal_method, internal);
            assert!(!route.legacy_alias);
        }
    }
}
