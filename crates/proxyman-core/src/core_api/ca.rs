use serde::{Deserialize, Serialize};

use crate::core_api::{ensure_supported_version, CORE_API_VERSION};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaStatusRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaInstallRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaStatusResponse {
    pub(crate) api_version: u16,
    pub(crate) installed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaInstallResponse {
    pub(crate) api_version: u16,
    pub(crate) installed: bool,
}

pub(crate) fn validate_ca_status_request(request: &CaStatusRequest) -> Result<(), String> {
    ensure_supported_version(request.api_version)
}

pub(crate) fn validate_ca_install_request(request: &CaInstallRequest) -> Result<(), String> {
    ensure_supported_version(request.api_version)
}

pub(crate) fn ca_status_response(installed: bool) -> CaStatusResponse {
    CaStatusResponse {
        api_version: CORE_API_VERSION,
        installed,
    }
}

pub(crate) fn ca_install_response(installed: bool) -> CaInstallResponse {
    CaInstallResponse {
        api_version: CORE_API_VERSION,
        installed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::CORE_API_VERSION;

    #[test]
    fn core_api_ca_rejects_unsupported_status_version() {
        let err = validate_ca_status_request(&CaStatusRequest {
            api_version: CORE_API_VERSION + 1,
        })
        .expect_err("unsupported version should fail");

        assert_eq!(
            err,
            format!("Unsupported core API version: {}", CORE_API_VERSION + 1)
        );
    }

    #[test]
    fn core_api_ca_builds_status_and_install_responses() {
        assert_eq!(
            ca_status_response(true),
            CaStatusResponse {
                api_version: CORE_API_VERSION,
                installed: true,
            }
        );
        assert_eq!(
            ca_install_response(false),
            CaInstallResponse {
                api_version: CORE_API_VERSION,
                installed: false,
            }
        );
    }
}
