use async_trait::async_trait;
use http::Uri;
use hyper::{Body, Request};
use regex::{NoExpand, Regex};
use std::str::FromStr;

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
            for [source, dest] in mappings.iter() {
                let re = Regex::new(source.as_str()).unwrap();
                let uri = req.uri().to_string();

                if !re.is_match(&uri) {
                    continue;
                }

                let result = re.replace(uri.as_str(), NoExpand(dest));
                *req.uri_mut() = Uri::from_str((*result).as_ref()).unwrap();

                return (req.into(), true);
            }
        }

        (req.into(), false)
    }
}

impl ProcessorRuleParser for RequestRedirectProcessor {
    type Rule = Vec<[String; 2]>;

    /// Parse configuation like this:
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
