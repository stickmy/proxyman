use serde::ser::SerializeStructVariant;

#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Processor({0}) not found")]
    NotFound(String),
    #[error("Mode({0}) not defined")]
    ModeNotDefined(String),
    #[error("Unexpected error: {0}")]
    OtherError(String),
}

impl serde::Serialize for ProcessorError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::IoError(inner) => {
                let mut state =
                    serializer.serialize_struct_variant("ProcessorError", 0, "IoError", 1)?;
                state.serialize_field("message", inner.to_string().as_str())?;
                state.end()
            }
            Self::NotFound(name) => {
                let mut state =
                    serializer.serialize_struct_variant("ProcessorError", 1, "NotFound", 1)?;
                state.serialize_field("message", format!("{} not found", name).as_str())?;
                state.end()
            }
            Self::ModeNotDefined(name) => {
                let mut state = serializer.serialize_struct_variant(
                    "ProcessorError",
                    2,
                    "ModeNotDefined",
                    1,
                )?;
                state.serialize_field("message", format!("{} is not defined", name).as_str())?;
                state.end()
            }
            Self::OtherError(msg) => {
                let mut state =
                    serializer.serialize_struct_variant("ProcessorError", 3, "OtherError", 1)?;
                state.serialize_field("message", msg)?;
                state.end()
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessorPackError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ParseJsonError(#[from] serde_json::Error),
}

impl serde::Serialize for ProcessorPackError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::IoError(inner) => {
                let mut state =
                    serializer.serialize_struct_variant("ProcessorPackError", 0, "IoError", 1)?;
                state.serialize_field("message", inner.to_string().as_str())?;
                state.end()
            }
            Self::ParseJsonError(inner) => {
                let mut state = serializer.serialize_struct_variant(
                    "ProcessorPackError",
                    1,
                    "ParseJsonError",
                    1,
                )?;
                state.serialize_field(
                    "message",
                    format!("line: {}, column: {}", inner.line(), inner.column()).as_str(),
                )?;
                state.end()
            }
        }
    }
}
