use std::collections::HashMap;

use async_trait::async_trait;
use http::{HeaderName, HeaderValue, Uri};
use hyper::{Body, Request, Response};
use regex::Regex;

use crate::processors::{parser::ProcessorRuleParser, Processor};

use super::{
    HttpRequestProcessor, HttpResponseProcessor, ProcessorID, RequestProcessResult,
    ResponseProcessResult,
};

impl ProcessorID {
    pub const RESPONSE_HEADER: ProcessorID = ProcessorID("ResponseHeader");
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ResponseHeaderProcessor {
    mappings: Option<Vec<ResponseHeaderMapping>>,
}

#[derive(Debug, Clone)]
struct ResponseHeaderMapping {
    status_matcher: Regex,
    header_name: HeaderName,
    header_value: HeaderValue,
}

impl ResponseHeaderProcessor {
    pub fn set_mapping(&mut self, mapping: Vec<[String; 3]>) {
        self.mappings = compiled_response_header_mappings(mapping);
    }
}

impl Processor for ResponseHeaderProcessor {
    fn name(&self) -> ProcessorID {
        ProcessorID::RESPONSE_HEADER
    }
}

#[async_trait]
impl HttpRequestProcessor for ResponseHeaderProcessor {
    async fn process_request(&self, req: Request<Body>) -> RequestProcessResult {
        (req.into(), false, None)
    }
}

#[async_trait]
impl HttpResponseProcessor for ResponseHeaderProcessor {
    async fn process_response(&self, _req_uri: &Uri, mut res: Response<Body>) -> ResponseProcessResult {
        if let Some(ref mappings) = self.mappings {
            let status = res.status().as_u16().to_string();

            for mapping in mappings.iter() {
                if !mapping.status_matcher.is_match(&status) {
                    continue;
                }

                res.headers_mut()
                    .insert(mapping.header_name.clone(), mapping.header_value.clone());

                let mut hit_info = HashMap::<String, String>::new();
                hit_info.insert(String::from("status"), status);
                hit_info.insert(String::from("header"), mapping.header_name.to_string());
                hit_info.insert(
                    String::from("value"),
                    mapping
                        .header_value
                        .to_str()
                        .unwrap_or_default()
                        .to_string(),
                );

                return (res, true, Some(hit_info));
            }
        }

        (res, false, None)
    }
}

impl ProcessorRuleParser for ResponseHeaderProcessor {
    type Rule = Vec<[String; 3]>;

    fn parse_rule(content: &str) -> Self::Rule {
        content
            .lines()
            .filter_map(|line| parse_line(line.trim()))
            .collect()
    }
}

impl From<String> for ResponseHeaderProcessor {
    fn from(content: String) -> Self {
        if content.is_empty() {
            return ResponseHeaderProcessor::default();
        }

        ResponseHeaderProcessor {
            mappings: compiled_response_header_mappings(Self::parse_rule(content.as_str())),
        }
    }
}

fn parse_line(line: &str) -> Option<[String; 3]> {
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let mut parts = line.split_whitespace();
    let status = parts.next()?.to_string();
    let header = parts.next()?.to_string();
    let value = parts.collect::<Vec<_>>().join(" ");

    if value.is_empty() {
        return None;
    }

    Some([status, header, value])
}

fn compiled_response_header_mappings(
    mapping: Vec<[String; 3]>,
) -> Option<Vec<ResponseHeaderMapping>> {
    let mappings = mapping
        .into_iter()
        .filter_map(|[status_pattern, header_name, header_value]| {
            let status_matcher = match Regex::new(&status_pattern) {
                Ok(matcher) => matcher,
                Err(err) => {
                    log::debug!("invalid response header status regex({status_pattern}): {err}");
                    return None;
                }
            };
            let header_name = match HeaderName::try_from(header_name.as_str()) {
                Ok(header_name) => header_name,
                Err(err) => {
                    log::debug!("invalid response header name({header_name}): {err}");
                    return None;
                }
            };
            let header_value = match HeaderValue::try_from(header_value.as_str()) {
                Ok(header_value) => header_value,
                Err(err) => {
                    log::debug!("invalid response header value({header_value}): {err}");
                    return None;
                }
            };

            Some(ResponseHeaderMapping {
                status_matcher,
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

    #[test]
    fn rules_invalid_response_header_regex_is_rejected_at_build_time() {
        let processor = ResponseHeaderProcessor::from("[ x-test value".to_string());

        assert!(processor.mappings.is_none());
    }

    #[tokio::test]
    async fn rules_response_header_processor_updates_matching_response() {
        let processor = ResponseHeaderProcessor::from("2.. x-test value".to_string());
        let res = Response::builder()
            .status(200)
            .body(Body::empty())
            .expect("failed to build response");

        let req_uri = Uri::from_static("https://example.test/");
        let (res, hit, _) = processor.process_response(&req_uri, res).await;

        assert!(hit);
        assert_eq!(res.headers().get("x-test").unwrap(), "value");
    }
}
