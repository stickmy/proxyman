use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AppSetting {
    theme: String,
    layout: String,
}

impl Default for AppSetting {
    fn default() -> Self {
        Self {
            theme: String::from("dark"),
            layout: String::from("right"),
        }
    }
}

pub(super) fn read_app_setting<P: AsRef<Path>>(
    path: P,
) -> anyhow::Result<AppSetting, super::error::Error> {
    let content_raw = fs::read(path).map_err(super::error::Error::from)?;

    let content_str = String::from_utf8(content_raw).map_err(|err| {
        log::error!("Failed to convert app settings content to utf8: {err}");
        super::error::Error::Utf8(err)
    })?;

    let conf: AppSetting = serde_json::from_str(content_str.as_str()).map_err(|err| {
        log::error!("Failed to deserialized app settings content to json: {err}");
        super::error::Error::Json(err)
    })?;

    Ok(conf)
}

pub(super) fn write_app_setting<P: AsRef<Path>>(
    path: P,
    conf: AppSetting,
) -> Result<(), super::error::Error> {
    let str = serde_json::to_string(&conf).map_err(|err| {
        log::error!("Failed to serialize app settings content to json: {err}");
        super::error::Error::Json(err)
    })?;

    fs::write(path, str).map_err(super::error::Error::from)
}
