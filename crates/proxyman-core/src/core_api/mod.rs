pub(crate) mod ca;
pub(crate) mod events;
pub(crate) mod proxy;
pub(crate) mod rules;
pub(crate) mod session;
pub(crate) mod system_proxy;
pub(crate) mod upstream_tls;

pub(crate) const CORE_API_VERSION: u16 = 1;

pub(crate) fn ensure_supported_version(api_version: u16) -> Result<(), String> {
    if api_version == CORE_API_VERSION {
        Ok(())
    } else {
        Err(format!("Unsupported core API version: {api_version}"))
    }
}
