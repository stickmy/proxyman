use self::{
    configuration_error::ConfigurationErrorKind, endpoint_error::EndpointError,
    processor_error::ProcessorErrorKind,
};
use crate::processors::processor_id::ProcessorID;
use serde::{
    ser::{SerializeStruct, SerializeStructVariant},
    Serialize,
};
use serde_json::json;
use snafu::Snafu;

pub mod configuration_error;
pub mod endpoint_error;
pub mod processor_error;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Error)))]
pub enum Error {
    #[snafu(display("Configuration error: {}", scenario))]
    Configuration {
        scenario: &'static str,
        source: ConfigurationErrorKind,
    },
    #[snafu(display("Processor({}) error", id))]
    Processor {
        id: ProcessorID,
        source: ProcessorErrorKind,
    },
    #[snafu(display("ProcessorPack error"))]
    ProcessorPack { source: ProcessorErrorKind },
    #[snafu(display("Processor status error"))]
    ProcessorStatus { source: ProcessorErrorKind },
    #[snafu(display("Error occurred with the server in {}: {}", scenario, source))]
    Server {
        scenario: &'static str,
        source: EndpointError,
    },
    #[snafu(display("Error occurred with the client in {}: {}", scenario, source))]
    Client {
        scenario: &'static str,
        source: EndpointError,
    },
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Configuration { scenario, source } => {
                let mut state =
                    serializer.serialize_struct_variant("Error", 0, "Confirguation", 3)?;

                state.serialize_field("type", "ConfigurationError")?;
                state.serialize_field("message", *scenario)?;
                state.serialize_field("cause", source)?;
                state.end()
            }
            Self::ProcessorPack { source } => {
                let mut state = serializer.serialize_struct("Error", 2)?;

                state.serialize_field("type", "ProcessorError")?;
                state.serialize_field("cause", source)?;
                state.end()
            }
            Self::Processor { id, source } => {
                let mut state = serializer.serialize_struct("Error", 3)?;

                state.serialize_field("type", "ProcessorError")?;
                state.serialize_field("id", id)?;
                state.serialize_field("cause", source)?;
                state.end()
            }
            Self::ProcessorStatus { source } => {
                let mut state = serializer.serialize_struct("Error", 2)?;

                state.serialize_field("type", "ProcessorStatusError")?;
                state.serialize_field("cause", source)?;
                state.end()
            }
            Self::Client { scenario, source } => {
                let mut state = serializer.serialize_struct_variant("Error", 0, "Client", 3)?;

                state.serialize_field("type", "TunnelClientError")?;
                state.serialize_field("message", *scenario)?;
                state.serialize_field("cause", source)?;
                state.end()
            }
            Self::Server { scenario, source } => {
                let mut state = serializer.serialize_struct_variant("Error", 0, "Server", 2)?;

                state.serialize_field("message", *scenario)?;
                state.serialize_field("cause", source)?;
                state.end()
            }
        }
    }
}

impl Error {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or(
            json!({ "type": "Unknown", "message": "Serialize error struct failed" }).to_string(),
        )
    }
}
