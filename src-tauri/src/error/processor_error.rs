use serde::{ser::SerializeStruct, Serialize};
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Error)))]
pub enum ProcessorErrorKind {
    Write { source: std::io::Error },
    Read { source: std::io::Error },
    WriteStatus { source: std::io::Error },
    ReadStatus { source: std::io::Error },
    NotFound {},
    Fmt {},
    Unsupport {},
    Unknown {},
}

impl Serialize for ProcessorErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Write { source } => {
                let mut state = serializer.serialize_struct("Write", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::Read { source } => {
                let mut state = serializer.serialize_struct("Read", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::WriteStatus { source } => {
                let mut state = serializer.serialize_struct("WriteStatus", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::ReadStatus { source } => {
                let mut state = serializer.serialize_struct("ReadStatus", 1)?;
                state.serialize_field("message", source.to_string().as_str())?;
                state.end()
            }
            Self::Fmt {} => {
                let mut state = serializer.serialize_struct("Format", 1)?;
                state.serialize_field("message", "Format error")?;
                state.end()
            }
            Self::NotFound {} => {
                let mut state = serializer.serialize_struct("NotFound", 1)?;
                state.serialize_field("message", "Not found")?;
                state.end()
            }
            Self::Unsupport {} => {
                let mut state = serializer.serialize_struct("Unsupport", 1)?;
                state.serialize_field("message", "Unsupport processor")?;
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
