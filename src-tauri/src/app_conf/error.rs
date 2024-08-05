use serde::ser::SerializeStructVariant;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Io(inner) => {
                let mut sv =
                    serializer.serialize_struct_variant("AppSettingError", 0, "IoError", 1)?;
                sv.serialize_field("message", inner.to_string().as_str())?;
                sv.end()
            }
            Self::Utf8(inner) => {
                let mut sv =
                    serializer.serialize_struct_variant("AppSettingError", 1, "Utf8Error", 1)?;
                sv.serialize_field("message", inner.to_string().as_str())?;
                sv.end()
            }
            Self::Json(inner) => {
                let mut sv =
                    serializer.serialize_struct_variant("AppSettingError", 2, "JsonError", 1)?;
                sv.serialize_field("message", inner.to_string().as_str())?;
                sv.end()
            }
        }
    }
}
