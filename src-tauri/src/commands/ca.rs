use async_process::Command;
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

use crate::error::{ConfigurationErrorKind, Error};

#[tauri::command]
pub async fn check_cert_installed<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let ca_path = get_ca_path(app);

    let child = Command::new("security")
        .arg("verify-cert")
        .arg("-c")
        .arg(ca_path.as_os_str())
        .output()
        .await;

    Ok(child
        .ok()
        .and_then(|c| String::from_utf8(c.stdout).ok())
        .map_or(false, |output| {
            output.contains("certificate verification successful")
        }))
}

#[tauri::command]
pub async fn install_cert<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let key_chain = get_key_chain()
        .await
        .map_err(|_| "Read default-keychain failed".to_string())?;
    let ca_path = get_ca_path(app);

    let child = Command::new("security")
        .arg("add-trusted-cert")
        .arg("-k")
        .arg(key_chain.as_str())
        .arg(ca_path.as_os_str())
        .output()
        .await;

    child
        .map_err(|e| {
            log::error!(
                "Install certificate - keychain: {}, ca: {:?}, error: {}",
                key_chain,
                ca_path,
                e
            );
            "Install certificate failed".to_string()
        })
        .and_then(|out| {
            String::from_utf8(out.stdout).map_err(|_| "Install certificate failed".to_string())
        })
        .map(|ret| {
            log::debug!("Install cert output: {}", ret);
            !ret.contains("Error:")
        })
}

fn get_ca_path<R: Runtime>(app: AppHandle<R>) -> PathBuf {
    app.path_resolver()
        .resolve_resource("ca/proxyman.cer")
        .expect("failed to resolve certificate")
}

async fn get_key_chain() -> Result<String, Error> {
    let child = Command::new("security")
        .arg("default-keychain")
        .output()
        .await;

    child
        .map_err(|e| Error::Configuration {
            reason: "Execute default-keychain command",
            source: ConfigurationErrorKind::Cert {},
        })
        .and_then(|c| {
            String::from_utf8(c.stdout).map_err(|e| Error::Configuration {
                reason: "Read default-keychain command output",
                source: ConfigurationErrorKind::Cert {},
            })
        })
        .and_then(|output| {
            let mut parts = output.split('"');

            parts.next();
            match parts.next() {
                Some(path) => Ok(path.to_string()),
                None => Err(Error::Configuration {
                    reason: "Read default-keychain command output",
                    source: ConfigurationErrorKind::Cert {},
                }),
            }
        })
}
