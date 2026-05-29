use std::collections::HashMap;

use async_trait::async_trait;
use http::Uri;
use hyper::{Body, Request, Response};

use crate::{
    processors::processor::{self, RequestOrResponse},
    processors::processor_effect::{ProcessorEffect, ProcessorEffects},
};

use super::{processor_id::ProcessorID, processor_pack::ProcessorPack, Processor};

use self::{
    delay::RequestDelayProcessor, redirect::RequestRedirectProcessor,
    request_header::RequestHeaderProcessor, response::ResponseProcessor,
};

pub mod delay;
pub mod redirect;
pub mod request_header;
pub mod response;
pub mod response_header;
pub mod typed_rules;

pub type RequestProcessResult = (
    RequestOrResponse,
    // hit or not
    bool,
    // hit info
    Option<HashMap<String, String>>,
);
pub type ResponseProcessResult = (
    Response<Body>,
    // hit or not
    bool,
    // hit info
    Option<HashMap<String, String>>,
);

#[async_trait]
pub trait HttpRequestProcessor: Send + Sync + std::fmt::Debug {
    async fn process_request(&self, req: Request<Body>) -> RequestProcessResult;
}

#[cfg(test)]
mod tests {
    use super::{
        typed_rules::TypedRuleProcessor, HttpProcessor, HttpRequestProcessor as _,
        HttpResponseProcessor as _,
    };
    use crate::{
        core_api::rules::{
            parse_typed_rules_dsl, HeaderMutation, RuleAction, RuleMatcher, RulePhase,
            TextMatcher, TypedRuleDefinition,
        },
        processors::{processor::HttpProcessor as _, processor_pack::ProcessorPack},
    };
    use http::Uri;
    use hyper::{body::to_bytes, Body, Request, Response};
    use tokio::time::{Duration, Instant};

    #[tokio::test]
    async fn typed_rules_runtime_redirects_request() {
        let processor = TypedRuleProcessor::from_rules(
            parse_typed_rules_dsl(
                "redirect GET https://api.example.com/users* https://mock.local/users",
            )
            .expect("rules should parse"),
        )
        .expect("rules should compile");
        let req = Request::builder()
            .method("GET")
            .uri("https://api.example.com/users/1")
            .body(Body::empty())
            .expect("request should build");

        let (req_or_res, hit, info) = processor.process_request(req).await;

        assert!(hit);
        assert_eq!(req_or_res.req.uri(), "https://mock.local/users");
        assert!(info
            .and_then(|info| info.get("ruleId").cloned())
            .is_some_and(|rule_id| rule_id.starts_with("dsl-1-")));
    }

    #[tokio::test]
    async fn typed_rules_runtime_sets_request_header() {
        let processor = TypedRuleProcessor::from_rules(
            parse_typed_rules_dsl("request-header GET https://api.example.com/* x-debug true")
                .expect("rules should parse"),
        )
        .expect("rules should compile");
        let req = Request::builder()
            .method("GET")
            .uri("https://api.example.com/users")
            .body(Body::empty())
            .expect("request should build");

        let (req_or_res, hit, _) = processor.process_request(req).await;

        assert!(hit);
        assert_eq!(req_or_res.req.headers().get("x-debug").unwrap(), "true");
    }

    #[tokio::test]
    async fn typed_rules_runtime_applies_multiple_request_actions() {
        let processor = TypedRuleProcessor::from_rules(vec![TypedRuleDefinition {
            id: "multi-request".to_string(),
            name: None,
            enabled: true,
            priority: 0,
            phase: RulePhase::Request,
            matchers: vec![RuleMatcher::Path {
                matcher: TextMatcher::Contains {
                    value: "/users".to_string(),
                },
            }],
            actions: vec![
                RuleAction::RequestHeader {
                    operation: HeaderMutation::Set,
                    name: "x-debug".to_string(),
                    value: Some("true".to_string()),
                },
                RuleAction::RequestBody {
                    body: "patched request body".to_string(),
                },
            ],
        }])
        .expect("rules should compile");
        let req = Request::builder()
            .method("POST")
            .uri("https://api.example.com/users")
            .body(Body::from("original request body"))
            .expect("request should build");

        let (req_or_res, hit, info) = processor.process_request(req).await;

        assert!(hit);
        assert_eq!(req_or_res.req.headers().get("x-debug").unwrap(), "true");
        assert_eq!(
            info.and_then(|info| info.get("actions").cloned()),
            Some("requestHeader,requestBody".to_string())
        );
        let body = to_bytes(req_or_res.req.into_body())
            .await
            .expect("request body should read");
        assert_eq!(body.as_ref(), b"patched request body");
    }

    #[tokio::test]
    async fn typed_rules_runtime_maps_local_file_without_upstream() {
        let path = std::env::temp_dir()
            .join(format!("proxyman-map-local-{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&path, "local response body").expect("fixture should write");
        let rule = format!(
            "map-local GET https://static.example.com/* {}",
            path.to_string_lossy()
        );
        let processor = TypedRuleProcessor::from_rules(
            parse_typed_rules_dsl(rule.as_str()).expect("rules should parse"),
        )
        .expect("rules should compile");
        let req = Request::builder()
            .method("GET")
            .uri("https://static.example.com/app.js")
            .body(Body::empty())
            .expect("request should build");

        let (req_or_res, hit, info) = processor.process_request(req).await;
        let _ = std::fs::remove_file(&path);

        assert!(hit);
        assert_eq!(req_or_res.res.as_ref().unwrap().status(), 200);
        assert_eq!(
            info.and_then(|info| info.get("action").cloned()),
            Some("mapLocal".to_string())
        );
        let body = to_bytes(req_or_res.res.unwrap().into_body())
            .await
            .expect("response body should read");
        assert_eq!(body.as_ref(), b"local response body");
    }

    #[tokio::test]
    async fn typed_rules_runtime_blocks_request() {
        let processor = TypedRuleProcessor::from_rules(
            parse_typed_rules_dsl("block * https://ads.example.com/* 403 blocked")
                .expect("rules should parse"),
        )
        .expect("rules should compile");
        let req = Request::builder()
            .method("GET")
            .uri("https://ads.example.com/banner")
            .body(Body::empty())
            .expect("request should build");

        let (req_or_res, hit, _) = processor.process_request(req).await;

        assert!(hit);
        assert_eq!(req_or_res.res.as_ref().unwrap().status(), 403);
    }

    #[tokio::test]
    async fn typed_rules_runtime_delays_matching_request() {
        let processor = TypedRuleProcessor::from_rules(
            parse_typed_rules_dsl("delay * https://api.example.com/slow* 20ms")
                .expect("rules should parse"),
        )
        .expect("rules should compile");
        let req = Request::builder()
            .method("GET")
            .uri("https://api.example.com/slow")
            .body(Body::empty())
            .expect("request should build");
        let started = Instant::now();

        let (_, hit, _) = processor.process_request(req).await;

        assert!(hit);
        assert!(started.elapsed() >= Duration::from_millis(20));
    }

    #[tokio::test]
    async fn typed_rules_runtime_runs_before_legacy_processors_in_pack() {
        let mut pack = ProcessorPack::new("test".to_string(), true);
        pack.set_typed_rules(
            TypedRuleProcessor::from_rules(
                parse_typed_rules_dsl(
                    "redirect GET https://api.example.com/users* https://mock.local/users",
                )
                .expect("rules should parse"),
            )
            .expect("rules should compile"),
        );
        pack.set_request_header(super::request_header::RequestHeaderProcessor::from(
            "GET .* x-legacy should-not-run".to_string(),
        ));
        let processor = HttpProcessor::new(vec![pack]);
        let req = Request::builder()
            .method("GET")
            .uri("https://api.example.com/users/1")
            .body(Body::empty())
            .expect("request should build");

        let req_or_res = processor.process_request(req).await;

        assert_eq!(req_or_res.req.uri(), "https://mock.local/users");
        assert!(req_or_res.req.headers().get("x-legacy").is_none());
    }

    #[tokio::test]
    async fn typed_rules_runtime_sets_response_header_with_url_match() {
        let processor = TypedRuleProcessor::from_rules(
            parse_typed_rules_dsl("response-header 200 https://api.example.com/* x-cache hit")
                .expect("rules should parse"),
        )
        .expect("rules should compile");
        let res = Response::builder()
            .status(200)
            .body(Body::empty())
            .expect("response should build");
        let req_uri = Uri::from_static("https://api.example.com/users");

        let (res, hit, info) = processor.process_response(&req_uri, res).await;

        assert!(hit);
        assert_eq!(res.headers().get("x-cache").unwrap(), "hit");
        assert_eq!(
            info.and_then(|info| info.get("action").cloned()),
            Some("responseHeader".to_string())
        );
    }

    #[tokio::test]
    async fn typed_rules_runtime_replaces_response_body() {
        let processor = TypedRuleProcessor::from_rules(vec![TypedRuleDefinition {
            id: "response-body".to_string(),
            name: None,
            enabled: true,
            priority: 0,
            phase: RulePhase::Response,
            matchers: vec![RuleMatcher::Status { value: 200 }],
            actions: vec![RuleAction::ResponseBody {
                body: "patched response body".to_string(),
            }],
        }])
        .expect("rules should compile");
        let res = Response::builder()
            .status(200)
            .body(Body::from("origin response body"))
            .expect("response should build");
        let req_uri = Uri::from_static("https://api.example.com/users");

        let (res, hit, _) = processor.process_response(&req_uri, res).await;

        assert!(hit);
        let body = to_bytes(res.into_body())
            .await
            .expect("response body should read");
        assert_eq!(body.as_ref(), b"patched response body");
    }

    #[tokio::test]
    async fn typed_rules_runtime_blocks_matching_response() {
        let processor = TypedRuleProcessor::from_rules(vec![TypedRuleDefinition {
            id: "response-block".to_string(),
            name: None,
            enabled: true,
            priority: 0,
            phase: RulePhase::Response,
            matchers: vec![RuleMatcher::Status { value: 200 }],
            actions: vec![RuleAction::Block {
                status: 503,
                body: Some("blocked response".to_string()),
            }],
        }])
        .expect("rules should compile");
        let res = Response::builder()
            .status(200)
            .body(Body::from("origin response body"))
            .expect("response should build");
        let req_uri = Uri::from_static("https://api.example.com/users");

        let (res, hit, info) = processor.process_response(&req_uri, res).await;

        assert!(hit);
        assert_eq!(res.status(), 503);
        assert_eq!(
            info.and_then(|info| info.get("action").cloned()),
            Some("block".to_string())
        );
        let body = to_bytes(res.into_body())
            .await
            .expect("response body should read");
        assert_eq!(body.as_ref(), b"blocked response");
    }

    #[tokio::test]
    async fn typed_rules_runtime_skips_response_header_for_unmatched_url() {
        let processor = TypedRuleProcessor::from_rules(
            parse_typed_rules_dsl("response-header 200 https://api.example.com/* x-cache hit")
                .expect("rules should parse"),
        )
        .expect("rules should compile");
        let res = Response::builder()
            .status(200)
            .body(Body::empty())
            .expect("response should build");
        let req_uri = Uri::from_static("https://other.example.com/users");

        let (res, hit, _) = processor.process_response(&req_uri, res).await;

        assert!(!hit);
        assert!(res.headers().get("x-cache").is_none());
    }
}

#[async_trait]
pub trait HttpResponseProcessor: Send + Sync + std::fmt::Debug + Processor {
    async fn process_response(&self, req_uri: &Uri, res: Response<Body>) -> ResponseProcessResult;
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

    pub(crate) fn get_response_header_mut(
        &mut self,
        pack_name: String,
    ) -> Option<&mut response_header::ResponseHeaderProcessor> {
        for pack in self.packs.iter_mut() {
            if pack.pack_name == pack_name {
                return Some(pack.get_response_header_mut());
            }
        }

        None
    }

    pub(crate) fn get_request_header_mut(
        &mut self,
        pack_name: String,
    ) -> Option<&mut RequestHeaderProcessor> {
        for pack in self.packs.iter_mut() {
            if pack.pack_name == pack_name {
                return Some(pack.get_request_header_mut());
            }
        }

        None
    }
}

#[async_trait]
impl processor::HttpProcessor for HttpProcessor {
    async fn process_request(&self, req: Request<Body>) -> RequestOrResponse {
        let mut processed_ret: RequestOrResponse = req.into();
        let mut effects: ProcessorEffects = HashMap::new();

        for pack in self.packs.iter() {
            if pack.is_enable() {
                // process request
                let processors: Vec<&dyn Processor> = vec![
                    pack.get_typed_rules(),
                    pack.get_redirect(),
                    pack.get_request_header(),
                    pack.get_response(),
                    pack.get_delay(),
                ];

                let mut pack_effect = Vec::<ProcessorEffect>::new();

                // Match all rules that could be matched in a single pack.
                for processor in processors.iter() {
                    let (req_or_res, hit, info) =
                        processor.process_request(processed_ret.req).await;

                    log::trace!(
                        "process_request result: pack({}), processor({}), hit({hit}), info({:?})",
                        pack.pack_name,
                        processor.name(),
                        info
                    );

                    if hit {
                        let stop_after_typed_rule = processor.name() == ProcessorID::TYPED_RULES;
                        pack_effect.push(ProcessorEffect {
                            name: processor.name(),
                            info,
                        });

                        processed_ret.req = req_or_res.req;
                        if let Some(res) = req_or_res.res {
                            processed_ret.res = Some(res);
                        }
                        if stop_after_typed_rule {
                            break;
                        }
                    } else {
                        processed_ret.req = req_or_res.req;
                        if let Some(res) = req_or_res.res {
                            processed_ret.res = Some(res);
                        }
                    }
                }

                if !pack_effect.is_empty() {
                    effects.insert(pack.pack_name.clone(), pack_effect);
                    // Break the matching if any a pack could be matched successfully.
                    break;
                }
            }
        }

        if !effects.is_empty() {
            processed_ret.processor_effects = Some(effects);
        }

        processed_ret
    }

    async fn process_response(&self, req_uri: &Uri, res: Response<Body>) -> Response<Body> {
        let mut processed_res = res;

        for pack in self.packs.iter() {
            if pack.is_enable() {
                let processors: Vec<&dyn HttpResponseProcessor> =
                    vec![pack.get_typed_rules(), pack.get_response_header()];

                for processor in processors.iter() {
                    let (res, hit, info) =
                        processor.process_response(req_uri, processed_res).await;

                    log::trace!(
                        "process_response result: pack({}), processor({}), hit({hit}), info({:?})",
                        pack.pack_name,
                        processor.name(),
                        info
                    );

                    processed_res = res;

                    if hit {
                        return processed_res;
                    }
                }
            }
        }

        processed_res
    }
}
