use std::{
    fs,
    io::ErrorKind,
    path::Path,
};

use hyper::{client::HttpConnector, Body, Client, Request, Response};
use hyper_tls::{native_tls, HttpsConnector};
use serde_json::{Map, Value};
use tokio_tungstenite::Connector;

use crate::app_conf;

const IGNORE_UPSTREAM_TLS_VERIFICATION_KEY: &str = "ignoreUpstreamTlsVerification";

type SystemTrustHttpsConnector = HttpsConnector<HttpConnector>;
type SystemTrustHttpsConnectorResult = Result<SystemTrustHttpsConnector, native_tls::Error>;

#[derive(Clone)]
pub(crate) struct UpstreamHttpClient {
    system_trust_client: Client<SystemTrustHttpsConnector>,
    insecure_client: Client<SystemTrustHttpsConnector>,
}

impl UpstreamHttpClient {
    pub(crate) fn new() -> Result<Self, native_tls::Error> {
        Ok(Self {
            system_trust_client: upstream_client(false)?,
            insecure_client: upstream_client(true)?,
        })
    }

    pub(crate) async fn request(
        &self,
        request: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        if ignore_upstream_tls_verification_enabled() {
            self.insecure_client.request(request).await
        } else {
            self.system_trust_client.request(request).await
        }
    }
}

pub(crate) fn ignore_upstream_tls_verification_enabled() -> bool {
    read_ignore_upstream_tls_verification().unwrap_or(false)
}

pub(crate) fn read_ignore_upstream_tls_verification() -> Result<bool, String> {
    read_ignore_upstream_tls_verification_at(app_conf::app_settings_file().as_path())
}

pub(crate) fn write_ignore_upstream_tls_verification(enabled: bool) -> Result<bool, String> {
    write_ignore_upstream_tls_verification_at(app_conf::app_settings_file().as_path(), enabled)
}

fn https_connector(ignore_upstream_tls_verification: bool) -> SystemTrustHttpsConnectorResult {
    let mut http = HttpConnector::new();
    http.enforce_http(false);

    let tls = native_tls_connector(ignore_upstream_tls_verification)?;
    Ok(HttpsConnector::from((http, tls.into())))
}

pub(crate) fn websocket_connector() -> Result<Connector, native_tls::Error> {
    Ok(Connector::NativeTls(native_tls_connector(
        ignore_upstream_tls_verification_enabled(),
    )?))
}

fn native_tls_connector(
    ignore_upstream_tls_verification: bool,
) -> Result<native_tls::TlsConnector, native_tls::Error> {
    let mut tls_builder = native_tls::TlsConnector::builder();
    tls_builder
        .danger_accept_invalid_certs(ignore_upstream_tls_verification)
        .danger_accept_invalid_hostnames(ignore_upstream_tls_verification);
    tls_builder.build()
}

fn upstream_client(
    ignore_upstream_tls_verification: bool,
) -> Result<Client<SystemTrustHttpsConnector>, native_tls::Error> {
    Ok(Client::builder()
        .http1_preserve_header_case(true)
        .http1_title_case_headers(true)
        .build(https_connector(ignore_upstream_tls_verification)?))
}

fn read_ignore_upstream_tls_verification_at(path: &Path) -> Result<bool, String> {
    Ok(read_settings_object(path)?
        .get(IGNORE_UPSTREAM_TLS_VERIFICATION_KEY)
        .and_then(Value::as_bool)
        .unwrap_or(false))
}

fn write_ignore_upstream_tls_verification_at(path: &Path, enabled: bool) -> Result<bool, String> {
    let mut settings = read_settings_object(path)?;
    settings.insert(
        IGNORE_UPSTREAM_TLS_VERIFICATION_KEY.to_string(),
        Value::Bool(enabled),
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Create settings directory failed: {err}"))?;
    }

    let content = serde_json::to_vec_pretty(&settings)
        .map_err(|err| format!("Serialize settings failed: {err}"))?;
    fs::write(path, content).map_err(|err| format!("Write settings failed: {err}"))?;

    Ok(enabled)
}

fn read_settings_object(path: &Path) -> Result<Map<String, Value>, String> {
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Map::new()),
        Err(err) => return Err(format!("Read settings failed: {err}")),
    };

    if content.is_empty() {
        return Ok(Map::new());
    }

    match serde_json::from_slice::<Value>(content.as_slice())
        .map_err(|err| format!("Parse settings failed: {err}"))?
    {
        Value::Object(settings) => Ok(settings),
        _ => Err("Parse settings failed: root value must be an object".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_tls_settings_default_to_system_trust() {
        let dir = tempfile::tempdir().expect("tempdir should create");
        let path = dir.path().join("missing-settings.json");

        assert!(!read_ignore_upstream_tls_verification_at(path.as_path())
            .expect("missing settings should read"));
    }

    #[test]
    fn upstream_tls_settings_preserve_existing_settings() {
        let dir = tempfile::tempdir().expect("tempdir should create");
        let path = dir.path().join("settings.json");
        fs::write(
            path.as_path(),
            br#"{"theme_name":"macOS Classic Light","layout":"right"}"#,
        )
        .expect("settings should write");

        assert!(write_ignore_upstream_tls_verification_at(path.as_path(), true)
            .expect("settings should update"));

        let settings = read_settings_object(path.as_path()).expect("settings should read");
        assert_eq!(
            settings.get("theme_name").and_then(Value::as_str),
            Some("macOS Classic Light")
        );
        assert_eq!(
            settings
                .get(IGNORE_UPSTREAM_TLS_VERIFICATION_KEY)
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
