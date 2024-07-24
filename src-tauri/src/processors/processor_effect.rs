use std::collections::HashMap;

use serde::Serialize;

use crate::processors::processor_id::ProcessorID;

pub type ProcessorEffects = HashMap<String /* pack name */, Vec<ProcessorEffect> /* pack hit rules */>;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessorEffect {
    pub name: ProcessorID,
    pub info: Option<HashMap<String, String>>,
}