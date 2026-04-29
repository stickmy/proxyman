use serde::{Deserialize, Serialize};

use crate::core_api::{ensure_supported_version, CORE_API_VERSION};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnableSystemProxyRequest {
    pub(crate) api_version: u16,
    pub(crate) port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DisableSystemProxyRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemProxyStatusRequest {
    pub(crate) api_version: u16,
    pub(crate) host: Option<String>,
    pub(crate) port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemProxyResponse {
    pub(crate) api_version: u16,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemProxyState {
    pub(crate) enabled: bool,
    pub(crate) server: Option<String>,
    pub(crate) port: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemProxyServiceStatus {
    pub(crate) service: String,
    pub(crate) web: SystemProxyState,
    pub(crate) secure_web: SystemProxyState,
    pub(crate) bypass_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemProxyStatusResponse {
    pub(crate) api_version: u16,
    pub(crate) enabled: bool,
    pub(crate) matches_requested: bool,
    pub(crate) services: Vec<SystemProxyServiceStatus>,
}

pub(crate) fn validate_enable_system_proxy_request(
    request: &EnableSystemProxyRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;

    if request.port == 0 {
        return Err("System proxy port must be greater than 0".to_string());
    }

    Ok(())
}

pub(crate) fn validate_disable_system_proxy_request(
    request: &DisableSystemProxyRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)
}

pub(crate) fn validate_system_proxy_status_request(
    request: &SystemProxyStatusRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;

    if request.host.as_deref().is_some_and(str::is_empty) {
        return Err("System proxy host must not be empty".to_string());
    }
    if request.port == Some(0) {
        return Err("System proxy port must be greater than 0".to_string());
    }

    Ok(())
}

pub(crate) fn system_proxy_response(enabled: bool) -> SystemProxyResponse {
    SystemProxyResponse {
        api_version: CORE_API_VERSION,
        enabled,
    }
}

pub(crate) fn system_proxy_status_response(
    services: Vec<SystemProxyServiceStatus>,
    host: Option<&str>,
    port: Option<u16>,
) -> SystemProxyStatusResponse {
    let enabled = services
        .iter()
        .any(|service| service.web.enabled || service.secure_web.enabled);
    let matches_requested = match (host, port) {
        (Some(host), Some(port)) if !services.is_empty() => services.iter().all(|service| {
            proxy_state_matches(&service.web, host, port)
                && proxy_state_matches(&service.secure_web, host, port)
        }),
        _ => false,
    };

    SystemProxyStatusResponse {
        api_version: CORE_API_VERSION,
        enabled,
        matches_requested,
        services,
    }
}

fn proxy_state_matches(state: &SystemProxyState, host: &str, port: u16) -> bool {
    let port = port.to_string();
    state.enabled
        && state.server.as_deref() == Some(host)
        && state.port.as_deref() == Some(port.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::CORE_API_VERSION;

    #[test]
    fn core_api_system_proxy_rejects_unsupported_api_version() {
        let err = validate_enable_system_proxy_request(&EnableSystemProxyRequest {
            api_version: CORE_API_VERSION + 1,
            port: 8888,
        })
        .expect_err("unsupported version should fail");

        assert_eq!(
            err,
            format!("Unsupported core API version: {}", CORE_API_VERSION + 1)
        );
    }

    #[test]
    fn core_api_system_proxy_rejects_ephemeral_port_zero() {
        let err = validate_enable_system_proxy_request(&EnableSystemProxyRequest {
            api_version: CORE_API_VERSION,
            port: 0,
        })
        .expect_err("port zero should fail");

        assert_eq!(err, "System proxy port must be greater than 0");
    }

    #[test]
    fn core_api_system_proxy_rejects_invalid_status_target() {
        let err = validate_system_proxy_status_request(&SystemProxyStatusRequest {
            api_version: CORE_API_VERSION,
            host: Some("127.0.0.1".to_string()),
            port: Some(0),
        })
        .expect_err("port zero should fail");

        assert_eq!(err, "System proxy port must be greater than 0");
    }

    #[test]
    fn core_api_system_proxy_builds_typed_response() {
        assert_eq!(
            system_proxy_response(true),
            SystemProxyResponse {
                api_version: CORE_API_VERSION,
                enabled: true,
            }
        );
    }

    #[test]
    fn core_api_system_proxy_builds_status_response() {
        let service = SystemProxyServiceStatus {
            service: "Wi-Fi".to_string(),
            web: SystemProxyState {
                enabled: true,
                server: Some("127.0.0.1".to_string()),
                port: Some("9000".to_string()),
            },
            secure_web: SystemProxyState {
                enabled: true,
                server: Some("127.0.0.1".to_string()),
                port: Some("9000".to_string()),
            },
            bypass_domains: vec!["localhost".to_string()],
        };

        assert_eq!(
            system_proxy_status_response(vec![service.clone()], Some("127.0.0.1"), Some(9000)),
            SystemProxyStatusResponse {
                api_version: CORE_API_VERSION,
                enabled: true,
                matches_requested: true,
                services: vec![service],
            }
        );
    }
}
