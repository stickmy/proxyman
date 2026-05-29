use async_trait::async_trait;
use std::time::Duration;

use regex::Regex;
use tokio::time::sleep;

use super::{HttpRequestProcessor, ProcessorID, RequestProcessResult};
use crate::processors::{parser::ProcessorRuleParser, Processor};

impl ProcessorID {
    pub const DELAY: ProcessorID = ProcessorID("Delay");
}

#[derive(Debug, Clone)]
pub(crate) struct RequestDelayMapping {
    pub req_pattern: String,
    pub delay_millsec: u64,
}

pub(crate) type RequestDelayRule = Vec<RequestDelayMapping>;

#[derive(Debug, Clone, Default)]
pub(crate) struct RequestDelayProcessor {
    mappings: Option<Vec<CompiledRequestDelayMapping>>,
}

#[derive(Debug, Clone)]
struct CompiledRequestDelayMapping {
    matcher: Regex,
    delay_millsec: u64,
}

impl RequestDelayProcessor {
    pub fn set_delay_mapping(&mut self, mappings: RequestDelayRule) {
        self.mappings = compiled_delay_mappings(mappings);
    }
}

impl Processor for RequestDelayProcessor {
    fn name(&self) -> ProcessorID {
        ProcessorID::DELAY
    }
}

#[async_trait]
impl HttpRequestProcessor for RequestDelayProcessor {
    async fn process_request(&self, req: http::Request<hyper::Body>) -> RequestProcessResult {
        if let Some(ref mappings) = self.mappings {
            for mapping in mappings.iter() {
                let uri = req.uri().to_string();

                if !mapping.matcher.is_match(&uri) {
                    continue;
                }

                let delay_millsec = mapping.delay_millsec;

                tokio::spawn(async move {
                    sleep(Duration::from_millis(delay_millsec)).await;
                })
                .await
                .unwrap();

                return (req.into(), true, None);
            }
        }

        (req.into(), false, None)
    }
}

impl ProcessorRuleParser for RequestDelayProcessor {
    type Rule = RequestDelayRule;

    /// Parse configuration like this:
    /// ```shell
    /// # strip this line
    /// https://www.x.com 200
    ///    https://wwww.m.com  300
    /// ```
    fn parse_rule(content: &str) -> Self::Rule {
        let lines = content.split('\n');

        let mut mappings: RequestDelayRule = Vec::new();

        for line in lines.into_iter() {
            let line = line.trim();

            if !line.starts_with('#') {
                let mut parts = line.split_whitespace();
                let mut req_pattern = String::new();

                if let Some(source) = parts.next() {
                    req_pattern = source.into();
                }

                if let Some(delay_str) = parts.next() {
                    let delay = delay_str.parse::<u64>();

                    match delay {
                        Ok(delay) => {
                            mappings.push(RequestDelayMapping {
                                req_pattern,
                                delay_millsec: delay,
                            });
                        }
                        Err(_) => {
                            log::error!("parse delay - {} failed", delay_str);
                        }
                    }
                }
            }
        }

        mappings
    }
}

impl From<String> for RequestDelayProcessor {
    fn from(value: String) -> Self {
        if value.is_empty() {
            return RequestDelayProcessor::default();
        }

        let mappings = compiled_delay_mappings(RequestDelayProcessor::parse_rule(value.as_str()));

        RequestDelayProcessor { mappings }
    }
}

fn compiled_delay_mappings(mappings: RequestDelayRule) -> Option<Vec<CompiledRequestDelayMapping>> {
    let mappings = mappings
        .into_iter()
        .filter_map(|mapping| match Regex::new(&mapping.req_pattern) {
            Ok(matcher) => Some(CompiledRequestDelayMapping {
                matcher,
                delay_millsec: mapping.delay_millsec,
            }),
            Err(err) => {
                log::debug!("invalid delay regex({}): {err}", mapping.req_pattern);
                None
            }
        })
        .collect::<Vec<_>>();

    if mappings.is_empty() {
        None
    } else {
        Some(mappings)
    }
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;
    use hyper::{Body, Request};

    use super::*;

    #[tokio::test]
    async fn correctness_invalid_delay_regex_does_not_panic() {
        let mut processor = RequestDelayProcessor::default();
        processor.set_delay_mapping(vec![RequestDelayMapping {
            req_pattern: "[".to_string(),
            delay_millsec: 1,
        }]);
        let req = Request::builder()
            .uri("https://example.test/path")
            .body(Body::empty())
            .expect("failed to build request");

        let result = std::panic::AssertUnwindSafe(processor.process_request(req))
            .catch_unwind()
            .await;

        assert!(result.is_ok());
        let (_, hit, _) = result.unwrap();
        assert!(!hit);
    }

    #[test]
    fn rules_invalid_delay_regex_is_rejected_at_build_time() {
        let processor = RequestDelayProcessor::from("[ 1".to_string());

        assert!(processor.mappings.is_none());
    }
}
