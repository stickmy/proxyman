use std::{fs, path::PathBuf};

use self::app_setting::{read_app_setting, write_app_setting};

pub(crate) use self::app_setting::AppSetting;

mod app_setting;
pub mod error;

pub fn init() -> Result<(), self::error::Error> {
    ensure_app_dir()
}

pub fn app_logger_file() -> PathBuf {
    get_app_path("proxyman.log")
}

pub fn app_rule_dir() -> PathBuf {
    get_app_path("rule")
}

pub fn app_value_dir() -> PathBuf {
    get_app_path("value")
}

pub fn app_processor_pack_status_file() -> PathBuf {
    get_app_path("processor_pack_status.json")
}

fn app_setting_file() -> PathBuf {
    get_app_path("settings.json")
}

pub fn get_app_setting() -> AppSetting {
    read_app_setting(app_setting_file()).unwrap_or_default()
}

pub fn save_app_setting(conf: AppSetting) -> Result<(), self::error::Error> {
    write_app_setting(app_setting_file(), conf)
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

fn ensure_app_dir() -> Result<(), self::error::Error> {
    let app_dir = app_dir();

    match fs::metadata(&app_dir) {
        Ok(meta) => {
            if !meta.is_dir() {
                fs::create_dir(app_dir).map_err(self::error::Error::from)
            } else {
                Ok(())
            }
        }
        Err(_) => fs::create_dir(app_dir).map_err(self::error::Error::from),
    }
}
