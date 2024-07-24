use core::fmt;

use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ProcessorID(pub(super) &'static str);

impl fmt::Display for ProcessorID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(clippy::from_over_into)]
impl Into<&str> for ProcessorID {
    fn into(self) -> &'static str {
        self.0
    }
}

#[allow(clippy::from_over_into)]
impl Into<String> for ProcessorID {
    fn into(self) -> String {
        self.0.to_string()
    }
}

impl TryFrom<String> for ProcessorID {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Delay" => Ok(Self::DELAY),
            "Redirect" => Ok(Self::REDIRECT),
            "Response" => Ok(Self::RESPONSE),
            _ => Err("Unsupport processor"),
        }
    }
}

impl Serialize for ProcessorID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0)
    }
}
