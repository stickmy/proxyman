use async_trait::async_trait;
use hyper::{Body, Request, Response};

use crate::proxy::processor::{self, RequestOrResponse};

use self::{
    delay::RequestDelayProcessor, redirect::RequestRedirectProcessor, response::ResponseProcessor,
};

use super::{processor_id::ProcessorID, processor_pack::ProcessorPack, Processor};

pub mod delay;
pub mod redirect;
pub mod response;

#[async_trait]
pub trait HttpRequestProcessor: Send + Sync + std::fmt::Debug {
    async fn process_request(&self, req: Request<Body>) -> (RequestOrResponse, bool);
}

#[async_trait]
pub trait HttpResponseProcessor: Send + Sync + std::fmt::Debug + Processor {
    async fn process_response(&self, res: Response<Body>) -> (Response<Body>, bool);
}

#[derive(Debug, Clone)]
pub struct HttpProcessor {
    pub(crate) packs: Vec<ProcessorPack>,
}

impl HttpProcessor {
    pub fn new(packs: Vec<ProcessorPack>) -> Self {
        Self { packs }
    }

    pub fn disable_pack(&mut self, pack_name: String) {
        for pack in self.packs.iter_mut() {
            if pack.pack_name == pack_name {
                pack.disbale()
            }
        }
    }

    pub fn enable_pack(&mut self, pack_name: String) {
        for pack in self.packs.iter_mut() {
            if pack.pack_name == pack_name {
                pack.enable()
            }
        }
    }

    pub fn add_pack(&mut self, pack: ProcessorPack) {
        self.packs.push(pack);
    }

    pub fn remove_pack(&mut self, pack_name: String) {
        self.packs.retain(|x| x.pack_name != pack_name);
    }

    pub(crate) fn get_redirect_mut(
        &mut self,
        pack_name: String,
    ) -> Option<&mut RequestRedirectProcessor> {
        for pack in self.packs.iter_mut() {
            if pack.pack_name == pack_name {
                return Some(pack.get_redirect_mut());
            }
        }

        None
    }

    pub(crate) fn get_delay_mut(
        &mut self,
        pack_name: String,
    ) -> Option<&mut RequestDelayProcessor> {
        for pack in self.packs.iter_mut() {
            if pack.pack_name == pack_name {
                return Some(pack.get_delay_mut());
            }
        }

        None
    }

    pub(crate) fn get_response_mut(&mut self, pack_name: String) -> Option<&mut ResponseProcessor> {
        for pack in self.packs.iter_mut() {
            if pack.pack_name == pack_name {
                return Some(pack.get_response_mut());
            }
        }

        None
    }
}

#[async_trait]
impl processor::HttpProcessor for HttpProcessor {
    async fn process_request(&self, req: Request<Body>) -> RequestOrResponse {
        let mut processed_ret: RequestOrResponse = req.into();
        let mut hit_rules: Vec<String> = Vec::new();

        for pack in self.packs.iter() {
            // process request
            let processors: Vec<&dyn Processor> =
                vec![pack.get_response(), pack.get_redirect(), pack.get_delay()];

            for processor in processors.iter() {
                let (req_or_res, caught) = processor.process_request(processed_ret.req).await;

                if caught {
                    // TODO: 是否要加入 pack 信息
                    hit_rules.push(processor.name().0.to_string());
                }

                processed_ret.req = req_or_res.req;
                if let Some(res) = req_or_res.res {
                    processed_ret.res = Some(res);
                }
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
