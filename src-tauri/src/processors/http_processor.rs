use std::collections::HashMap;

use async_trait::async_trait;
use hyper::{Body, Request, Response};

use crate::proxy::processor::{self, RequestOrResponse, RuleHit};

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
        let mut hit_rules: RuleHit = HashMap::new();

        for pack in self.packs.iter() {
            if pack.is_enable() {
                // process request
                let processors: Vec<&dyn Processor> =
                    vec![pack.get_redirect(), pack.get_response(), pack.get_delay()];

                let mut pack_hit_rules = Vec::<String>::new();

                // Match all rules that could be matched in a signle pack.
                for processor in processors.iter() {
                    let (req_or_res, caught) = processor.process_request(processed_ret.req).await;

                    if caught {
                        pack_hit_rules.push(processor.name().0.to_string());
                    }

                    processed_ret.req = req_or_res.req;
                    if let Some(res) = req_or_res.res {
                        processed_ret.res = Some(res);
                    }
                }

                if !pack_hit_rules.is_empty() {
                    hit_rules.insert(pack.pack_name.clone(), pack_hit_rules);
                    // Break the matching if any a pack could be matched successfully.
                    break;
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
