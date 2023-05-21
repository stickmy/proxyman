use snafu::ResultExt;
use std::{fs, path::PathBuf};

use crate::error::{self, AppConfError, ConfigurationError};

pub fn init() -> Result<(), error::Error> {
    ensure_app_dir()
}

pub fn app_logger_file() -> PathBuf {
    get_app_path("proxyman.log")
}

pub fn app_rule_dir() -> PathBuf {
    get_app_path("rule")
}

fn get_app_path(name: &str) -> PathBuf {
    let mut path = app_dir();
    path.push(name);
    path
}

fn app_dir() -> PathBuf {
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
                fs::create_dir(app_dir)
                    .context(AppConfError {})
                    .context(ConfigurationError {
                        reason: "Ensure app dir",
                    })
            } else {
                Ok(())
            }
        }
        Err(_) => fs::create_dir(app_dir)
            .context(AppConfError {})
            .context(ConfigurationError {
                reason: "Ensure app dir",
            }),
    }
}
