use std::{collections::HashMap, str::FromStr};

use crate::{processors::Processor, proxy::processor::RequestOrResponse};
use async_trait::async_trait;

use http::{header::CONTENT_LENGTH, HeaderName, HeaderValue, StatusCode, Version};
use hyper::{Body, Request, Response};
use regex::Regex;

use super::{HttpRequestProcessor, ProcessorID, ProcessorRuleParser};

impl ProcessorID {
    pub const RESPONSE: ProcessorID = ProcessorID("Response");
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ResponseMapping {
    pub req_pattern: String,
    pub version: Version,
    pub status_code: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ResponseProcessor {
    mappings: Option<Vec<ResponseMapping>>,
}

impl ResponseProcessor {
    pub fn append_mapping(&mut self, mapping: ResponseMapping) {
        if self.mappings.is_none() {
            self.mappings = Some(Vec::new());
        }

        if let Some(ref mut mappings) = self.mappings {
            mappings.push(mapping)
        }
    }

    pub fn remove_mapping(&mut self, req_pattern: String) {
        if let Some(ref mut mappings) = self.mappings {
            for i in 0..mappings.len() {
                if mappings[i].req_pattern == req_pattern {
                    mappings.remove(i);
                }
            }
        }
    }
}

impl Processor for ResponseProcessor {
    fn name(&self) -> ProcessorID {
        ProcessorID::RESPONSE
    }
}

#[async_trait]
impl HttpRequestProcessor for ResponseProcessor {
    async fn process_request(&self, req: Request<Body>) -> (RequestOrResponse, bool) {
        if let Some(mappings) = self.clone().mappings {
            for ResponseMapping {
                req_pattern,
                version,
                status_code,
                headers,
                body,
            } in mappings.into_iter()
            {
                let re = Regex::new(&req_pattern).unwrap();
                if !re.is_match(&req.uri().to_string()) {
                    continue;
                }

                let mut builder = Response::builder().version(version).status(status_code);
                let header_map = builder.headers_mut().unwrap();
                for (k, v) in headers.iter() {
                    header_map.insert(
                        HeaderName::from_str(k).unwrap(),
                        HeaderValue::from_str(v).unwrap(),
                    );
                }
                let mut res = builder.body(Body::from(body)).unwrap();
                res.headers_mut().remove(CONTENT_LENGTH);

                return ((req, res).into(), true);
            }
        }

        (req.into(), false)
    }
}

impl ProcessorRuleParser for ResponseProcessor {
    type Rule = Option<ResponseMapping>;

    fn parse_rule(content: &str) -> Self::Rule {
        #[derive(Debug, PartialEq)]
        enum ParseState {
            ReqPattern,
            VersionStatus,
            Headers,
            BodyStart,
            Body,
        }

        let lines = content.split('\n');
        let mut state = ParseState::ReqPattern;
        let mut mapping = ResponseMapping::default();

        for line in lines.into_iter() {
            if line.is_empty() && state != ParseState::Body {
                state = ParseState::BodyStart;
                continue;
            }

            match state {
                ParseState::ReqPattern => {
                    mapping.req_pattern = line.into();
                    state = ParseState::VersionStatus;
                }
                ParseState::VersionStatus => {
                    let mut parts = line.split_ascii_whitespace();

                    let version = parts.next()?;
                    mapping.version = match version {
                        "HTTP/0.9" => Version::HTTP_09,
                        "HTTP/1.0" => Version::HTTP_10,
                        "HTTP/1.1" => Version::HTTP_11,
                        "HTTP/2.0" => Version::HTTP_2,
                        "HTTP/3.0" => Version::HTTP_3,
                        _ => return None,
                    };

                    let status_code = parts.next()?;
                    mapping.status_code = StatusCode::try_from(status_code).ok()?;

                    state = ParseState::Headers;
                }
                ParseState::Headers => {
                    let mut parts = line.split(':');
                    let key = parts.next()?.trim();
                    let value = parts.next()?.trim();
                    mapping.headers.insert(key.into(), value.into());
                }
                ParseState::BodyStart => {
                    mapping.body.push_str(line);
                    state = ParseState::Body;
                }
                ParseState::Body => {
                    mapping.body.push('\n');
                    mapping.body.push_str(line);
                }
            }
        }

        Some(mapping)
    }
}
