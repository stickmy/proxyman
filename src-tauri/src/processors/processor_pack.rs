use super::http_processor::{delay::*,redirect::*,response::*};

#[derive(Debug, Clone)]
pub struct ProcessorPack {
    pub pack_name: String,
    enable: bool,
    redirect: RequestRedirectProcessor,
    delay: RequestDelayProcessor,
    response: ResponseProcessor,
}

impl ProcessorPack {
    pub(crate) fn new(pack_name: String) -> Self {
        Self {
            pack_name,
            enable: false,
            redirect: RequestRedirectProcessor::default(),
            delay: RequestDelayProcessor::default(),
            response: ResponseProcessor::default(),
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

    pub(crate) fn get_response_mut(&mut self) -> &mut ResponseProcessor {
        &mut self.response
    }

    pub(crate) fn set_redirect(&mut self, redirect: RequestRedirectProcessor) {
        self.redirect = redirect;
    }

    pub(crate) fn set_delay(&mut self, delay: RequestDelayProcessor) {
        self.delay = delay
    }

    pub(crate) fn set_response(&mut self, response: ResponseProcessor) {
        self.response = response;
    }
}
