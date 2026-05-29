use serde::{Deserialize, Serialize};

use crate::core_api::{ensure_supported_version, CORE_API_VERSION};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpstreamTlsStatusRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateUpstreamTlsRequest {
    pub(crate) api_version: u16,
    pub(crate) ignore_verification: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpstreamTlsStatusResponse {
    pub(crate) api_version: u16,
    pub(crate) ignore_verification: bool,
}

pub(crate) fn validate_upstream_tls_status_request(
    request: &UpstreamTlsStatusRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)
}

pub(crate) fn validate_update_upstream_tls_request(
    request: &UpdateUpstreamTlsRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)
}

pub(crate) fn upstream_tls_status_response(
    ignore_verification: bool,
) -> UpstreamTlsStatusResponse {
    UpstreamTlsStatusResponse {
        api_version: CORE_API_VERSION,
        ignore_verification,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::CORE_API_VERSION;

    #[test]
    fn core_api_upstream_tls_rejects_unsupported_status_version() {
        let err = validate_upstream_tls_status_request(&UpstreamTlsStatusRequest {
            api_version: CORE_API_VERSION + 1,
        })
        .expect_err("unsupported version should fail");

        assert_eq!(
            err,
            format!("Unsupported core API version: {}", CORE_API_VERSION + 1)
        );
    }

    #[test]
    fn core_api_upstream_tls_builds_status_response() {
        assert_eq!(
            upstream_tls_status_response(true),
            UpstreamTlsStatusResponse {
                api_version: CORE_API_VERSION,
                ignore_verification: true,
            }
        );
    }
}
