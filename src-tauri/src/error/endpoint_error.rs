use serde::{ser::SerializeStruct, Serialize};
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Error)))]
pub enum EndpointError {
    Io {
        source: std::io::Error,
    },
    Connect {
        source: hyper::Error,
    },
    Http {
        source: hyper::Error,
    },
    Uri {
        source: http::uri::InvalidUri,
    },
    UriParts {
        source: http::uri::InvalidUriParts,
    },
    WebsocketProtocol {
        source: hyper_tungstenite::tungstenite::error::ProtocolError,
    },
    #[snafu(display("Error occurred when decoding {}", scenario))]
    Decoder {
        scenario: &'static str,
    },
    // #[snafu(display("{}", reason))]
    // Proxyman {
    //     reason: &'static str,
    // },
}

impl Serialize for EndpointError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Io { source } => {
                let mut state = serializer.serialize_struct("Io", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::Connect { source } => {
                let mut state = serializer.serialize_struct("Connect", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::Http { source } => {
                let mut state = serializer.serialize_struct("Http", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::Uri { source } => {
                let mut state = serializer.serialize_struct("Uri", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::UriParts { source } => {
                let mut state = serializer.serialize_struct("UriParts", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::WebsocketProtocol { source } => {
                let mut state = serializer.serialize_struct("WebsocketProtocol", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::Decoder { scenario } => {
                let mut state = serializer.serialize_struct("WebsocketProtocol", 1)?;
                state.serialize_field("message", *scenario)?;
                state.end()
            }
        }
    }
}
