use core::fmt;

#[derive(Debug, Eq, PartialEq)]
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
