use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use http::Uri;
use hyper::{Body, Request};
use regex::Regex;

use crate::processors::{parser::ProcessorRuleParser, Processor};

use super::{HttpRequestProcessor, ProcessorID, RequestProcessResult};

impl ProcessorID {
    pub const REDIRECT: ProcessorID = ProcessorID("Redirect");
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RequestRedirectProcessor {
    mappings: Option<Vec<RequestRedirectMapping>>,
}

#[derive(Clone, Debug)]
struct RequestRedirectMapping {
    matcher: Regex,
    dest: String,
}

impl RequestRedirectProcessor {
    pub fn set_redirects_mapping(&mut self, mapping: Vec<[String; 2]>) {
        self.mappings = compiled_redirect_mappings(mapping);
    }
}

impl Processor for RequestRedirectProcessor {
    fn name(&self) -> ProcessorID {
        ProcessorID::REDIRECT
    }
}

#[async_trait]
impl HttpRequestProcessor for RequestRedirectProcessor {
    async fn process_request(&self, mut req: Request<Body>) -> RequestProcessResult {
        if let Some(ref mappings) = self.mappings {
            for mapping in mappings.iter() {
                match replace_with_regex(&mapping.matcher, &mapping.dest, req.uri().to_string()) {
                    None => continue,
                    Some(ret) => {
                        let uri = match Uri::from_str(&ret) {
                            Ok(uri) => uri,
                            Err(err) => {
                                log::debug!("invalid redirect destination URI({ret}): {err}");
                                continue;
                            }
                        };
                        *req.uri_mut() = uri;

                        let mut hit_info = HashMap::<String, String>::new();
                        hit_info.insert(String::from("uri"), ret);

                        return (req.into(), true, Some(hit_info));
                    }
                }
            }
        }

        (req.into(), false, None)
    }
}

impl ProcessorRuleParser for RequestRedirectProcessor {
    type Rule = Vec<[String; 2]>;

    /// Parse configuration like this:
    /// ```shell
    /// ## This is a comment line
    /// https://www.x.com https://www.y.com
    /// https://wwww.m.com https://www.n.com
    /// ```
    fn parse_rule(content: &str) -> Self::Rule {
        let lines = content.split('\n');

        let mut mappings: Vec<[String; 2]> = Vec::new();

        for line in lines.into_iter() {
            let line = line.trim();

            if !line.starts_with('#') {
                let mut parts = line.split_whitespace();
                let mut mapping: [String; 2] = [String::new(), String::new()];

                if let Some(source) = parts.next() {
                    mapping[0] = source.into();
                }

                if let Some(dest) = parts.next() {
                    mapping[1] = dest.into();

                    mappings.push(mapping);
                }
            }
        }

        mappings
    }
}

impl From<String> for RequestRedirectProcessor {
    fn from(content: String) -> Self {
        if content.is_empty() {
            return RequestRedirectProcessor::default();
        }

        let mappings = compiled_redirect_mappings(Self::parse_rule(content.as_str()));

        RequestRedirectProcessor { mappings }
    }
}

fn compiled_redirect_mappings(mapping: Vec<[String; 2]>) -> Option<Vec<RequestRedirectMapping>> {
    let mappings = mapping
        .into_iter()
        .filter_map(|[reg_str, dest]| match Regex::new(&reg_str) {
            Ok(matcher) => Some(RequestRedirectMapping { matcher, dest }),
            Err(err) => {
                log::debug!("invalid redirect regex({reg_str}): {err}");
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

fn replace_with_regex(re: &Regex, dest: &str, source: String) -> Option<String> {
    match re.is_match(&source) {
        false => None,
        true => Some(re.replace(source.as_str(), dest).to_string()),
    }
}

fn replace_with_reg_str(reg_str: &str, dest: &String, source: String) -> Option<String> {
    let re = match Regex::new(reg_str) {
        Ok(re) => re,
        Err(err) => {
            log::debug!("invalid redirect regex({reg_str}): {err}");
            return None;
        }
    };

    match re.is_match(&source) {
        false => None,
        true => Some(re.replace(source.as_str(), dest).to_string()),
    }
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;
    use hyper::{Body, Request};

    use super::*;

    #[test]
    fn test_replace_with_reg_str() {
        let reg_str = "https://www.google.com/(.*)";
        let dest = "https://www.baidu.com/$1";
        let source = "https://www.google.com/a=1&b=2";

        let result = replace_with_reg_str(reg_str, &dest.to_string(), source.to_string());

        assert_eq!(result, Some("https://www.baidu.com/a=1&b=2".to_string()));
    }

    #[test]
    fn correctness_invalid_redirect_regex_does_not_panic() {
        let result = std::panic::catch_unwind(|| {
            replace_with_reg_str(
                "[",
                &"https://example.com".to_string(),
                "https://source.test".to_string(),
            )
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn rules_invalid_redirect_regex_is_rejected_at_build_time() {
        let processor =
            RequestRedirectProcessor::from("[ https://example.test/replacement".to_string());

        assert!(processor.mappings.is_none());
    }

    #[tokio::test]
    async fn correctness_invalid_redirect_destination_does_not_panic() {
        let mut processor = RequestRedirectProcessor::default();
        processor.set_redirects_mapping(vec![[
            "https://source.test/(.*)".to_string(),
            ":// invalid uri".to_string(),
        ]]);
        let req = Request::builder()
            .uri("https://source.test/path")
            .body(Body::empty())
            .expect("failed to build request");

        let result = std::panic::AssertUnwindSafe(processor.process_request(req))
            .catch_unwind()
            .await;

        assert!(result.is_ok());
        let (_, hit, _) = result.unwrap();
        assert!(!hit);
    }
}
