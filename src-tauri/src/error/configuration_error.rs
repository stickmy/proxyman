use serde::{ser::SerializeStruct, Serialize};
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Error)))]
pub enum ConfigurationErrorKind {
    Ssl { source: openssl::error::ErrorStack },
    AppConf { source: std::io::Error },
    Cert { scenario: &'static str },
    Unknown {},
}

impl Serialize for ConfigurationErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Ssl { source } => {
                let mut state = serializer.serialize_struct("Ssl", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::AppConf { source } => {
                let mut state = serializer.serialize_struct("AppConf", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::Cert { scenario } => {
                let mut state = serializer.serialize_struct("Certificate", 1)?;
                state.serialize_field("message", scenario)?;
                state.end()
            }
            Self::Unknown {} => {
                let mut state = serializer.serialize_struct("Unknown", 1)?;
                state.serialize_field("message", "Unknown")?;
                state.end()
            }
        }
    }
}
