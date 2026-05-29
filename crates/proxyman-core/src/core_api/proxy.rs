use serde::{Deserialize, Serialize};

use crate::core_api::{ensure_supported_version, CORE_API_VERSION};

pub(crate) const DEFAULT_PROXY_LISTEN_HOST: &str = "127.0.0.1";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartProxyRequest {
    pub(crate) api_version: u16,
    pub(crate) port: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartProxyResponse {
    pub(crate) api_version: u16,
    pub(crate) listen_host: String,
    pub(crate) port: u16,
    pub(crate) running: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopProxyRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopProxyResponse {
    pub(crate) api_version: u16,
    pub(crate) running: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProxyStatusRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProxyStatusResponse {
    pub(crate) api_version: u16,
    pub(crate) running: bool,
    pub(crate) listen_host: Option<String>,
    pub(crate) port: Option<u16>,
}

pub(crate) fn validate_start_request(request: &StartProxyRequest) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;

    if request.port == 0 {
        return Err("Proxy port must be greater than 0".to_string());
    }

    Ok(())
}

pub(crate) fn validate_stop_request(request: &StopProxyRequest) -> Result<(), String> {
    ensure_supported_version(request.api_version)
}

pub(crate) fn validate_status_request(request: &ProxyStatusRequest) -> Result<(), String> {
    ensure_supported_version(request.api_version)
}

pub(crate) fn start_proxy_response(port: u16) -> StartProxyResponse {
    StartProxyResponse {
        api_version: CORE_API_VERSION,
        listen_host: DEFAULT_PROXY_LISTEN_HOST.to_string(),
        port,
        running: true,
    }
}

pub(crate) fn stop_proxy_response() -> StopProxyResponse {
    StopProxyResponse {
        api_version: CORE_API_VERSION,
        running: false,
    }
}

pub(crate) fn proxy_status_response(running: bool, port: Option<u16>) -> ProxyStatusResponse {
    ProxyStatusResponse {
        api_version: CORE_API_VERSION,
        running,
        listen_host: running.then(|| DEFAULT_PROXY_LISTEN_HOST.to_string()),
        port: running.then_some(port).flatten(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::CORE_API_VERSION;

    #[test]
    fn core_api_proxy_rejects_unsupported_api_version() {
        let err = validate_start_request(&StartProxyRequest {
            api_version: CORE_API_VERSION + 1,
            port: 8899,
        })
        .expect_err("unsupported version should fail");

        assert_eq!(
            err,
            format!("Unsupported core API version: {}", CORE_API_VERSION + 1)
        );
    }

    #[test]
    fn core_api_proxy_rejects_ephemeral_port_zero() {
        let err = validate_start_request(&StartProxyRequest {
            api_version: CORE_API_VERSION,
            port: 0,
        })
        .expect_err("port zero should fail");

        assert_eq!(err, "Proxy port must be greater than 0");
    }

    #[test]
    fn core_api_proxy_builds_typed_status_response() {
        assert_eq!(
            proxy_status_response(true, Some(8888)),
            ProxyStatusResponse {
                api_version: CORE_API_VERSION,
                running: true,
                listen_host: Some("127.0.0.1".to_string()),
                port: Some(8888),
            }
        );

        assert_eq!(
            proxy_status_response(false, None),
            ProxyStatusResponse {
                api_version: CORE_API_VERSION,
                running: false,
                listen_host: None,
                port: None,
            }
        );
    }
}
