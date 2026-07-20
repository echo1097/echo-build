use thiserror::Error;
use url::{Host, Url};

pub const CREDENTIAL_ENDPOINT_REQUIREMENT: &str = "OpenRouter credentials require HTTPS to openrouter.ai on port 443 or an HTTP(S) loopback endpoint";

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("{CREDENTIAL_ENDPOINT_REQUIREMENT}")]
pub struct CredentialEndpointError;

pub fn validate_credential_endpoint(endpoint: &str) -> Result<(), CredentialEndpointError> {
    let url = Url::parse(endpoint).map_err(|_| CredentialEndpointError)?;

    if !url.username().is_empty() || url.password().is_some() {
        return Err(CredentialEndpointError);
    }

    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(CredentialEndpointError);
    }

    let host = url.host().ok_or(CredentialEndpointError)?;
    if matches!(host, Host::Domain(domain) if domain.eq_ignore_ascii_case("openrouter.ai")) {
        if scheme == "https" && url.port_or_known_default() == Some(443) {
            return Ok(());
        }

        return Err(CredentialEndpointError);
    }

    let is_loopback = match host {
        Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(address) => address.is_loopback(),
        Host::Ipv6(address) => address.is_loopback(),
    };
    if is_loopback {
        return Ok(());
    }

    Err(CredentialEndpointError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_production_https_and_loopback_http() {
        for endpoint in [
            "https://openrouter.ai/api/v1",
            "https://OPENROUTER.AI:443/api/v1",
            "http://127.0.0.1:3000",
            "http://[::1]:3000",
            "http://localhost:3000",
            "https://localhost:3000",
        ] {
            assert_eq!(validate_credential_endpoint(endpoint), Ok(()), "{endpoint}");
        }
    }

    #[test]
    fn rejects_unsafe_or_malformed_endpoints() {
        for endpoint in [
            "http://openrouter.ai/api/v1",
            "https://openrouter.ai:8443/api/v1",
            "ftp://openrouter.ai/api/v1",
            "https://openrouter.ai.example.com/api/v1",
            "https://openrouter-ai.example/api/v1",
            "https://user@openrouter.ai/api/v1",
            "https://user:pass@openrouter.ai/api/v1",
            "http://192.168.1.10:3000",
            "openrouter.ai/api/v1",
            "not a url",
        ] {
            assert_eq!(
                validate_credential_endpoint(endpoint),
                Err(CredentialEndpointError),
                "{endpoint}"
            );
        }
    }

    #[test]
    fn errors_do_not_echo_the_endpoint() {
        let endpoint = "https://secret@example.com/key-in-path";
        let error = validate_credential_endpoint(endpoint)
            .unwrap_err()
            .to_string();

        assert_eq!(error, CREDENTIAL_ENDPOINT_REQUIREMENT);
        assert!(!error.contains("secret"));
        assert!(!error.contains("key-in-path"));
    }
}
