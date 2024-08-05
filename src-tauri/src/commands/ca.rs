use async_process::Command;
use serde::ser::SerializeStructVariant;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

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
            log::info!("security verify-cert output: {output}");
            output.contains("certificate verification successful")
        }))
}

#[tauri::command]
pub async fn install_cert<R: Runtime>(app: AppHandle<R>) -> Result<bool, CertError> {
    let key_chain = get_key_chain().await?;

    let ca_path = get_ca_path(app);

    let child = Command::new("security")
        .arg("add-trusted-cert")
        .arg("-d")
        .arg("-r")
        .arg("trustRoot")
        .arg("-k")
        .arg(key_chain.as_str())
        .arg(ca_path.as_os_str())
        .output()
        .await;

    child
        .map_err(|err| {
            log::error!("add-trusted-cert command's output: {err}");
            CertError::InstallCert(String::from("Execute add-trusted-cert command failed"))
        })
        .and_then(|out| {
            String::from_utf8(out.stdout).map_err(|err| {
                log::error!("Failed to convert output of add-trusted-cert to utf8: {err}");
                CertError::InstallCert(String::from(
                    "Failed to convert output of add-trusted-cert to utf8",
                ))
            })
        })
        .map(|ret| {
            log::info!("Install certificate output: {ret}");
            !ret.contains("Error:")
        })
}

fn get_ca_path<R: Runtime>(app: AppHandle<R>) -> PathBuf {
    app.path()
        .resolve("ca/proxyman.cer", tauri::path::BaseDirectory::Resource)
        .expect("failed to resolve certificate")
}

async fn get_key_chain() -> Result<String, CertError> {
    let child = Command::new("security")
        .arg("default-keychain")
        .output()
        .await;

    child
        .map_err(|err| {
            log::error!("Execute default-keychain command error: {}", err);
            CertError::ReadCert(String::from("Execute default-keychain command"))
        })
        .and_then(|c| {
            String::from_utf8(c.stdout).map_err(|err| {
                log::error!(
                    "Failed to convert the default-keychain command's output to utf8: {err}"
                );
                CertError::ReadCert(String::from(
                    "Failed to convert the default-keychain command's output to utf8",
                ))
            })
        })
        .and_then(|output| {
            let mut parts = output.split('"');

            parts.next();
            match parts.next() {
                Some(path) => Ok(path.to_string()),
                None => {
                    log::error!(
                        "The format of default-keychain command's output was invalid, output: {:#?}",
                        parts
                    );
                    Err(CertError::ReadCert(String::from(
                        "The format of default-keychain command's output was invalid",
                    )))
                }
            }
        })
}

#[derive(Debug, thiserror::Error)]
pub enum CertError {
    #[error("Read certificate error: {0}")]
    ReadCert(String),
    #[error("Install certificate error: {0}")]
    InstallCert(String),
}

impl serde::Serialize for CertError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::ReadCert(msg) => {
                let mut sv = serializer.serialize_struct_variant("CertError", 0, "ReadyCert", 0)?;

                sv.serialize_field("message", msg)?;
                sv.end()
            }
            Self::InstallCert(msg) => {
                let mut sv =
                    serializer.serialize_struct_variant("CertError", 1, "InstallCert", 0)?;

                sv.serialize_field("message", msg)?;
                sv.end()
            }
        }
    }
}
