use serde::ser::SerializeStructVariant;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Client(#[from] EndpointError),
    #[error(transparent)]
    Server(#[from] hyper::Error),
    #[error(transparent)]
    Websocket(#[from] tokio_tungstenite::tungstenite::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    #[error("Unsupport decoding: {0}")]
    UnsupportDecoding(String),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Client(inner) => {
                let mut sv =
                    serializer.serialize_struct_variant("TunnelError", 0, "ClientError", 1)?;
                sv.serialize_field("message", inner.to_string().as_str())?;
                sv.end()
            }
            Self::Server(inner) => {
                let mut sv =
                    serializer.serialize_struct_variant("TunnelError", 1, "ServerError", 1)?;
                sv.serialize_field("message", inner.to_string().as_str())?;
                sv.end()
            }
            Self::Websocket(inner) => {
                let mut sv =
                    serializer.serialize_struct_variant("TunnelError", 2, "WebsocketError", 1)?;
                sv.serialize_field("message", inner.to_string().as_str())?;
                sv.end()
            }
        }
    }
}

impl serde::Serialize for EndpointError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::UnsupportDecoding(decoding) => {
                let mut sv = serializer.serialize_struct_variant(
                    "EndpointError",
                    0,
                    "UnsupportDecodingError",
                    1,
                )?;
                sv.serialize_field("message", decoding)?;
                sv.end()
            }
        }
    }
}
