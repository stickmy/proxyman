use self::processor_id::ProcessorID;

pub mod http_processor;
pub mod processor_id;

pub trait Processor {
    fn name(&self) -> ProcessorID;
}
