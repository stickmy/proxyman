use async_process::Command;
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

use crate::error::{configuration_error::ConfigurationErrorKind, Error};

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
pub async fn install_cert<R: Runtime>(app: AppHandle<R>) -> Result<bool, Error> {
    let key_chain = get_key_chain().await?;

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
            Error::Configuration {
                scenario: "Install https certificate",
                source: ConfigurationErrorKind::Cert {
                    scenario: "Execute 'security add-trusted-cert'",
                },
            }
        })
        .and_then(|out| {
            String::from_utf8(out.stdout).map_err(|_| Error::Configuration {
                scenario: "Read https certificate installation output",
                source: ConfigurationErrorKind::Cert {
                    scenario: "Transform output to uft8",
                },
            })
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
        .map_err(|_| Error::Configuration {
            scenario: "Execute default-keychain command",
            source: ConfigurationErrorKind::Cert {
                scenario: "read default keychain store",
            },
        })
        .and_then(|c| {
            String::from_utf8(c.stdout).map_err(|_| Error::Configuration {
                scenario: "Read default-keychain command output",
                source: ConfigurationErrorKind::Cert {
                    scenario: "read default keychain store",
                },
            })
        })
        .and_then(|output| {
            let mut parts = output.split('"');

            parts.next();
            match parts.next() {
                Some(path) => Ok(path.to_string()),
                None => Err(Error::Configuration {
                    scenario: "Read default-keychain command output",
                    source: ConfigurationErrorKind::Cert {
                        scenario: "transform default keychain store to string",
                    },
                }),
            }
        })
}
