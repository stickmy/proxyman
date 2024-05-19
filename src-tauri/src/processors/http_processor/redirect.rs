use std::str::FromStr;

use async_trait::async_trait;
use http::Uri;
use hyper::{Body, Request};
use regex::Regex;

use crate::{processors::Processor, proxy::processor::RequestOrResponse};

use super::{HttpRequestProcessor, ProcessorID, ProcessorRuleParser};

impl ProcessorID {
    pub const REDIRECT: ProcessorID = ProcessorID("Redirect");
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RequestRedirectProcessor {
    mappings: Option<Vec<[String; 2]>>,
}

impl RequestRedirectProcessor {
    pub fn set_redirects_mapping(&mut self, mapping: Vec<[String; 2]>) {
        self.mappings = Some(mapping);
    }
}

impl Processor for RequestRedirectProcessor {
    fn name(&self) -> ProcessorID {
        ProcessorID::REDIRECT
    }
}

#[async_trait]
impl HttpRequestProcessor for RequestRedirectProcessor {
    async fn process_request(&self, mut req: Request<Body>) -> (RequestOrResponse, bool) {
        if let Some(ref mappings) = self.mappings {
            for [reg_str, dest] in mappings.iter() {
                match replace_with_reg_str(reg_str, dest, req.uri().to_string()) {
                    None => continue,
                    Some(ret) => {
                        *req.uri_mut() = Uri::from_str(&ret).unwrap();
                    }
                }

                return (req.into(), true);
            }
        }

        (req.into(), false)
    }
}

fn replace_with_reg_str(reg_str: &String, dest: &String, source: String) -> Option<String> {
    let re = Regex::new(reg_str.as_str()).unwrap();

    match re.is_match(&source) {
        false => None,
        true => Some(re.replace(source.as_str(), dest).to_string())
    }
}

impl ProcessorRuleParser for RequestRedirectProcessor {
    type Rule = Vec<[String; 2]>;

    /// Parse configuration like this:
    /// ```shell
    /// # strip this line
    /// https://www.x.com https://www.y.com
    ///    https://wwww.m.com  https://www.n.com
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

        let mappings = Self::parse_rule(content.as_str());

        RequestRedirectProcessor {
            mappings: if mappings.is_empty() {
                None
            } else {
                Some(mappings)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_with_reg_str() {
        let reg_str = "https://www.google.com/(.*)";
        let dest = "https://www.baidu.com/$1";
        let source = "https://www.google.com/a=1&b=2";

        let result = replace_with_reg_str(&reg_str.to_string(), &dest.to_string(), source.to_string());

        assert_eq!(result, Some("https://www.baidu.com/a=1&b=2".to_string()));
    }
}

