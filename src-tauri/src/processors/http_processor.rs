use async_trait::async_trait;
use hyper::{Body, Request, Response};

use crate::proxy::processor::{self, RequestOrResponse};

use self::{
    delay::RequestDelayProcessor, redirect::RequestRedirectProcessor, response::ResponseProcessor,
};

use super::{processor_id::ProcessorID, Processor};

pub mod delay;
pub mod redirect;
pub mod response;

pub trait ProcessorRuleParser {
    type Rule;
    fn parse_rule(content: &str) -> Self::Rule;
}

#[async_trait]
pub trait HttpRequestProcessor: Send + Sync + std::fmt::Debug + Processor {
    async fn process_request(&self, req: Request<Body>) -> (RequestOrResponse, bool);
}

#[async_trait]
pub trait HttpResponseProcessor: Send + Sync + std::fmt::Debug + Processor {
    async fn process_response(&self, res: Response<Body>) -> (Response<Body>, bool);
}

#[derive(Debug, Clone)]
pub struct HttpProcessor {
    pub(crate) redirect: Option<RequestRedirectProcessor>,
    pub(crate) delay: Option<RequestDelayProcessor>,
    pub(crate) response: Option<ResponseProcessor>,
}

impl HttpProcessor {
    pub fn builder() -> Self {
        Self {
            redirect: None,
            delay: None,
            response: None,
        }
    }

    pub(crate) fn set_redirect_processor(mut self, redirect: RequestRedirectProcessor) -> Self {
        self.redirect = Some(redirect);
        self
    }

    pub(crate) fn set_delay_processor(mut self, delay: RequestDelayProcessor) -> Self {
        self.delay = Some(delay);
        self
    }

    pub(crate) fn set_response_processor(mut self, response: ResponseProcessor) -> Self {
        self.response = Some(response);
        self
    }
}

#[async_trait]
impl processor::HttpProcessor for HttpProcessor {
    async fn process_request(&self, req: Request<Body>) -> RequestOrResponse {
        // --------------------- collect processores ---------------------
        let mut processors: Vec<&dyn HttpRequestProcessor> = Vec::new();
        if let Some(ref response) = self.response {
            processors.push(response);
        }
        if let Some(ref redirect) = self.redirect {
            processors.push(redirect);
        }
        if let Some(ref delay) = self.delay {
            processors.push(delay);
        }

        // process request
        let mut processed_ret: RequestOrResponse = req.into();
        let mut hit_rules: Vec<String> = Vec::new();

        for processor in processors.iter() {
            let (req_or_res, catched) = processor.process_request(processed_ret.req).await;

            if catched {
                hit_rules.push(processor.name().0.to_string());
            }

            processed_ret.req = req_or_res.req;
            if let Some(res) = req_or_res.res {
                processed_ret.res = Some(res);
            }
        }

        if !hit_rules.is_empty() {
            processed_ret.hit_rules = Some(hit_rules);
        }

        processed_ret
    }

    async fn process_response(&self, res: Response<Body>) -> Response<Body> {
        res
    }
}
