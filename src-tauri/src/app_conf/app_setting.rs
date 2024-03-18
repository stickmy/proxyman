use crate::error::{
    self,
    configuration_error::{AppConfIoError, ConfigurationErrorKind},
    ConfigurationError, Error,
};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AppSetting {
    theme: String,
}

impl Default for AppSetting {
    fn default() -> Self {
        Self {
            theme: String::from("dark"),
        }
    }
}

pub(super) fn read_app_setting<P: AsRef<Path>>(path: P) -> Result<AppSetting, error::Error> {
    let content_raw = fs::read(path)
        .context(AppConfIoError {})
        .context(ConfigurationError {
            scenario: "read file",
        })?;

    let content_str = String::from_utf8(content_raw).map_err(|err| Error::Configuration {
        scenario: "read as utf8",
        source: ConfigurationErrorKind::AppSettingFmt {
            msg: format!("{err}"),
        },
    })?;

    let conf: AppSetting =
        serde_json::from_str(content_str.as_str()).map_err(|err| Error::Configuration {
            scenario: "deserialize with serde_json",
            source: ConfigurationErrorKind::AppSettingFmt {
                msg: format!("{err}"),
            },
        })?;

    Ok(conf)
}

pub(super) fn write_app_setting<P: AsRef<Path>>(
    path: P,
    conf: AppSetting,
) -> Result<(), error::Error> {
    let str = serde_json::to_string(&conf).map_err(|err| Error::Configuration {
        scenario: "serialized as string",
        source: ConfigurationErrorKind::AppSettingFmt {
            msg: format!("{err}"),
        },
    })?;

    fs::write(path, str)
        .context(AppConfIoError {})
        .context(ConfigurationError {
            scenario: "write to disk",
        })
}
