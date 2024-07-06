use self::processor_id::ProcessorID;
use crate::processors::http_processor::HttpRequestProcessor;

pub mod http_processor;
pub mod parser;
pub mod persist;
pub mod processor_id;
pub mod processor_pack;

pub trait Processor: HttpRequestProcessor {
    fn name(&self) -> ProcessorID;
}
