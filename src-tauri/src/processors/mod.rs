use self::processor_id::ProcessorID;
use crate::processors::http_processor::HttpRequestProcessor;

pub mod http_processor;
pub mod parser;
pub mod processor_id;
pub mod processor_pack;

pub mod values;

pub mod persist;
pub mod processor;
pub mod processor_effect;

pub trait Processor: HttpRequestProcessor {
    fn name(&self) -> ProcessorID;
}
