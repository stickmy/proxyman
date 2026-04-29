use std::collections::HashMap;

use async_trait::async_trait;
use http::{HeaderName, HeaderValue};
use hyper::{Body, Request};
use regex::Regex;

use crate::processors::{parser::ProcessorRuleParser, Processor};

use super::{HttpRequestProcessor, ProcessorID, RequestProcessResult};

impl ProcessorID {
    pub const REQUEST_HEADER: ProcessorID = ProcessorID("RequestHeader");
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RequestHeaderProcessor {
    mappings: Option<Vec<RequestHeaderMapping>>,
}

#[derive(Debug, Clone)]
struct RequestHeaderMapping {
    method_matcher: Regex,
    uri_matcher: Regex,
    header_name: HeaderName,
    header_value: HeaderValue,
}

impl RequestHeaderProcessor {
    pub fn set_mapping(&mut self, mapping: Vec<[String; 4]>) {
        self.mappings = compiled_request_header_mappings(mapping);
    }
}

impl Processor for RequestHeaderProcessor {
    fn name(&self) -> ProcessorID {
        ProcessorID::REQUEST_HEADER
    }
}

#[async_trait]
impl HttpRequestProcessor for RequestHeaderProcessor {
    async fn process_request(&self, mut req: Request<Body>) -> RequestProcessResult {
        if let Some(ref mappings) = self.mappings {
            let method = req.method().as_str().to_string();
            let uri = req.uri().to_string();

            for mapping in mappings.iter() {
                if !mapping.method_matcher.is_match(method.as_str())
                    || !mapping.uri_matcher.is_match(uri.as_str())
                {
                    continue;
                }

                req.headers_mut()
                    .insert(mapping.header_name.clone(), mapping.header_value.clone());

                let mut hit_info = HashMap::<String, String>::new();
                hit_info.insert(String::from("method"), method);
                hit_info.insert(String::from("uri"), uri);
                hit_info.insert(String::from("header"), mapping.header_name.to_string());
                hit_info.insert(
                    String::from("value"),
                    mapping
                        .header_value
                        .to_str()
                        .unwrap_or_default()
                        .to_string(),
                );

                return (req.into(), true, Some(hit_info));
            }
        }

        (req.into(), false, None)
    }
}

impl ProcessorRuleParser for RequestHeaderProcessor {
    type Rule = Vec<[String; 4]>;

    fn parse_rule(content: &str) -> Self::Rule {
        content
            .lines()
            .filter_map(|line| parse_line(line.trim()))
            .collect()
    }
}

impl From<String> for RequestHeaderProcessor {
    fn from(content: String) -> Self {
        if content.is_empty() {
            return RequestHeaderProcessor::default();
        }

        RequestHeaderProcessor {
            mappings: compiled_request_header_mappings(Self::parse_rule(content.as_str())),
        }
    }
}

fn parse_line(line: &str) -> Option<[String; 4]> {
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let uri = parts.next()?.to_string();
    let header = parts.next()?.to_string();
    let value = parts.collect::<Vec<_>>().join(" ");

    if value.is_empty() {
        return None;
    }

    Some([method, uri, header, value])
}

fn compiled_request_header_mappings(
    mapping: Vec<[String; 4]>,
) -> Option<Vec<RequestHeaderMapping>> {
    let mappings = mapping
        .into_iter()
        .filter_map(|[method_pattern, uri_pattern, header_name, header_value]| {
            let method_matcher = match Regex::new(&method_pattern) {
                Ok(matcher) => matcher,
                Err(err) => {
                    log::debug!("invalid request header method regex({method_pattern}): {err}");
                    return None;
                }
            };
            let uri_matcher = match Regex::new(&uri_pattern) {
                Ok(matcher) => matcher,
                Err(err) => {
                    log::debug!("invalid request header uri regex({uri_pattern}): {err}");
                    return None;
                }
            };
            let header_name = match HeaderName::try_from(header_name.as_str()) {
                Ok(header_name) => header_name,
                Err(err) => {
                    log::debug!("invalid request header name({header_name}): {err}");
                    return None;
                }
            };
            let header_value = match HeaderValue::try_from(header_value.as_str()) {
                Ok(header_value) => header_value,
                Err(err) => {
                    log::debug!("invalid request header value({header_value}): {err}");
                    return None;
                }
            };

            Some(RequestHeaderMapping {
                method_matcher,
                uri_matcher,
                header_name,
                header_value,
            })
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
    use super::*;
    use hyper::{Body, Request};

    #[test]
    fn rules_invalid_request_header_regex_is_rejected_at_build_time() {
        let processor = RequestHeaderProcessor::from("[ .* x-test value".to_string());

        assert!(processor.mappings.is_none());
    }

    #[tokio::test]
    async fn rules_request_header_processor_updates_matching_request() {
        let processor = RequestHeaderProcessor::from("GET .* x-test value".to_string());
        let req = Request::builder()
            .method("GET")
            .uri("http://example.com/path")
            .body(Body::empty())
            .expect("failed to build request");

        let (req_or_res, hit, _) = processor.process_request(req).await;

        assert!(hit);
        assert_eq!(req_or_res.req.headers().get("x-test").unwrap(), "value");
    }

    #[tokio::test]
    async fn rules_request_header_processor_skips_unmatched_method() {
        let processor = RequestHeaderProcessor::from("POST .* x-test value".to_string());
        let req = Request::builder()
            .method("GET")
            .uri("http://example.com/path")
            .body(Body::empty())
            .expect("failed to build request");

        let (req_or_res, hit, _) = processor.process_request(req).await;

        assert!(!hit);
        assert!(req_or_res.req.headers().get("x-test").is_none());
    }
}
