use super::http_processor::response_header::*;
use super::http_processor::{delay::*, redirect::*, request_header::*, response::*, typed_rules::*};

#[derive(Debug, Clone)]
pub struct ProcessorPack {
    pub pack_name: String,
    enable: bool,
    redirect: RequestRedirectProcessor,
    delay: RequestDelayProcessor,
    response: ResponseProcessor,
    request_header: RequestHeaderProcessor,
    response_header: ResponseHeaderProcessor,
    typed_rules: TypedRuleProcessor,
}

impl ProcessorPack {
    pub(crate) fn new(pack_name: String, enable: bool) -> Self {
        Self {
            pack_name,
            enable,
            redirect: RequestRedirectProcessor::default(),
            delay: RequestDelayProcessor::default(),
            response: ResponseProcessor::default(),
            request_header: RequestHeaderProcessor::default(),
            response_header: ResponseHeaderProcessor::default(),
            typed_rules: TypedRuleProcessor::default(),
        }
    }

    pub(crate) fn is_enable(&self) -> bool {
        self.enable
    }

    pub(crate) fn enable(&mut self) {
        self.enable = true;
    }

    pub(crate) fn disbale(&mut self) {
        self.enable = false;
    }

    pub(crate) fn get_redirect(&self) -> &RequestRedirectProcessor {
        &self.redirect
    }

    pub(crate) fn get_typed_rules(&self) -> &TypedRuleProcessor {
        &self.typed_rules
    }

    pub(crate) fn get_redirect_mut(&mut self) -> &mut RequestRedirectProcessor {
        &mut self.redirect
    }

    pub(crate) fn get_delay(&self) -> &RequestDelayProcessor {
        &self.delay
    }

    pub(crate) fn get_delay_mut(&mut self) -> &mut RequestDelayProcessor {
        &mut self.delay
    }

    pub(crate) fn get_response(&self) -> &ResponseProcessor {
        &self.response
    }

    pub(crate) fn get_request_header(&self) -> &RequestHeaderProcessor {
        &self.request_header
    }

    pub(crate) fn get_request_header_mut(&mut self) -> &mut RequestHeaderProcessor {
        &mut self.request_header
    }

    pub(crate) fn get_response_mut(&mut self) -> &mut ResponseProcessor {
        &mut self.response
    }

    pub(crate) fn get_response_header(&self) -> &ResponseHeaderProcessor {
        &self.response_header
    }

    pub(crate) fn get_response_header_mut(&mut self) -> &mut ResponseHeaderProcessor {
        &mut self.response_header
    }

    pub(crate) fn set_redirect(&mut self, redirect: RequestRedirectProcessor) {
        self.redirect = redirect;
    }

    pub(crate) fn set_typed_rules(&mut self, typed_rules: TypedRuleProcessor) {
        self.typed_rules = typed_rules;
    }

    pub(crate) fn set_delay(&mut self, delay: RequestDelayProcessor) {
        self.delay = delay
    }

    pub(crate) fn set_response(&mut self, response: ResponseProcessor) {
        self.response = response;
    }

    pub(crate) fn set_request_header(&mut self, request_header: RequestHeaderProcessor) {
        self.request_header = request_header;
    }

    pub(crate) fn set_response_header(&mut self, response_header: ResponseHeaderProcessor) {
        self.response_header = response_header;
    }
}
