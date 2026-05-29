use snafu::ResultExt;
use std::{fs, path::PathBuf};

use crate::error::{self, configuration_error::AppConfIoError, ConfigurationError};

pub fn init() -> Result<(), error::Error> {
    ensure_app_dir()
}

pub fn app_rule_dir() -> PathBuf {
    get_app_path("rule")
}

pub fn app_value_dir() -> PathBuf {
    get_app_path("value")
}

pub fn app_ca_cert_file() -> PathBuf {
    get_app_path("ca/proxyman.cer")
}

pub fn app_ca_key_file() -> PathBuf {
    get_app_path("ca/proxyman.key")
}

pub fn app_processor_pack_status_file() -> PathBuf {
    get_app_path("processor_pack_status.json")
}

pub fn app_system_proxy_snapshot_file() -> PathBuf {
    get_app_path("system_proxy_snapshot.json")
}

pub fn app_settings_file() -> PathBuf {
    get_app_path("settings.json")
}

fn get_app_path(name: &str) -> PathBuf {
    let mut path = app_dir();
    path.push(name);
    path
}

pub fn app_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    const APP_DOT: &str = ".proxyman_multi_conf_debug";
    #[cfg(not(debug_assertions))]
    const APP_DOT: &str = ".proxyman";

    let mut app_dir = home::home_dir().unwrap();
    app_dir.push(APP_DOT);

    app_dir.clone()
}

fn ensure_app_dir() -> Result<(), error::Error> {
    let app_dir = app_dir();

    match fs::metadata(&app_dir) {
        Ok(meta) => {
            if !meta.is_dir() {
                fs::create_dir_all(app_dir)
                    .context(AppConfIoError {})
                    .context(ConfigurationError {
                        scenario: "Ensure app dir",
                    })
            } else {
                Ok(())
            }
        }
        Err(_) => fs::create_dir_all(app_dir)
            .context(AppConfIoError {})
            .context(ConfigurationError {
                scenario: "Ensure app dir",
            }),
    }
}
