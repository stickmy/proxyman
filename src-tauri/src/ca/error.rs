use serde::ser::SerializeStructVariant;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    OpenSsl(#[from] openssl::error::ErrorStack),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::OpenSsl(inner) => {
                let mut sv = serializer.serialize_struct_variant("TlsError", 0, "OpenSsl", 1)?;
                sv.serialize_field("message", inner.to_string().as_str())?;
                sv.end()
            }
        }
    }
}
