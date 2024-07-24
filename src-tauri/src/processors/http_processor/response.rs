use std::{collections::HashMap, str::FromStr};

use crate::processors::{persist::value_persist::read_value, Processor};
use async_trait::async_trait;

use crate::processors::parser::ProcessorRuleParser;
use http::{header::CONTENT_LENGTH, HeaderName, HeaderValue, StatusCode, Version};
use hyper::{Body, Request, Response};
use regex::Regex;

use super::{HttpRequestProcessor, ProcessorID, RequestProcessResult};

impl ProcessorID {
    pub const RESPONSE: ProcessorID = ProcessorID("Response");
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ResponseProcessor {
    mappings: Option<Vec<[String; 2]>>,
}

impl ResponseProcessor {
    pub fn set_mapping(&mut self, mapping: Vec<[String; 2]>) {
        self.mappings = Some(mapping);
    }
}

impl Processor for ResponseProcessor {
    fn name(&self) -> ProcessorID {
        ProcessorID::RESPONSE
    }
}

#[async_trait]
impl HttpRequestProcessor for ResponseProcessor {
    async fn process_request(&self, req: Request<Body>) -> RequestProcessResult {
        if let Some(ref mappings) = self.mappings {
            for [req_pattern, value_name] in mappings.iter() {
                let re = Regex::new(req_pattern).unwrap();

                if !re.is_match(&req.uri().to_string()) {
                    continue;
                }

                let value_content = read_value(value_name);

                match value_content {
                    Ok(value) => {
                        let parsed = parse_str_as_response(value.as_ref());

                        match parsed {
                            Some(parsed) => {
                                let mut response = Response::builder()
                                    .version(parsed.version)
                                    .status(parsed.status_code);

                                let headers = response.headers_mut().unwrap();
                                for (k, v) in parsed.headers.iter() {
                                    headers.insert(
                                        HeaderName::from_str(k).unwrap(),
                                        HeaderValue::from_str(v).unwrap(),
                                    );
                                }

                                let mut res = response.body(Body::from(parsed.body)).unwrap();
                                res.headers_mut().remove(CONTENT_LENGTH);

                                let mut hit_info = HashMap::<String, String>::new();
                                hit_info.insert(String::from("name"), value_name.clone());

                                return ((req, res).into(), true, Some(hit_info));
                            }
                            None => {
                                log::debug!("parse value({value_name}) as response error");
                                continue;
                            }
                        }
                    }
                    Err(err) => {
                        log::debug!("read value({value_name}) error: {err}");
                        continue;
                    }
                }
            }
        }

        (req.into(), false, None)
    }
}

impl ProcessorRuleParser for ResponseProcessor {
    type Rule = Vec<[String; 2]>;

    /// Parse configuration like this:
    /// ```shell
    /// ## This is a comment line
    /// https://www.x.com x-response-value.json
    /// https://www.y.com y-response-value
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

impl From<String> for ResponseProcessor {
    fn from(value: String) -> Self {
        if value.is_empty() {
            return ResponseProcessor::default();
        }

        let mappings = Self::parse_rule(value.as_str());

        ResponseProcessor {
            mappings: if mappings.is_empty() {
                None
            } else {
                Some(mappings)
            },
        }
    }
}

#[derive(Default)]
struct ParsedResponse<'a> {
    version: Version,
    status_code: StatusCode,
    headers: HashMap<&'a str, &'a str>,
    body: String,
}

fn parse_str_as_response(content: &str) -> Option<ParsedResponse> {
    #[derive(Debug, PartialEq)]
    enum ParseState {
        VersionStatus,
        Headers,
        BodyStart,
        Body,
    }

    let mut parsed_response = ParsedResponse::default();

    let lines = content.split('\n');
    let mut state = ParseState::VersionStatus;

    for line in lines.into_iter() {
        if line.is_empty() && state != ParseState::Body {
            state = ParseState::BodyStart;
            continue;
        }

        match state {
            ParseState::VersionStatus => {
                let mut parts = line.split_ascii_whitespace();

                let version = parts.next()?;
                parsed_response.version = match version {
                    "HTTP/0.9" => Version::HTTP_09,
                    "HTTP/1.0" => Version::HTTP_10,
                    "HTTP/1.1" => Version::HTTP_11,
                    "HTTP/2.0" => Version::HTTP_2,
                    "HTTP/3.0" => Version::HTTP_3,
                    _ => return None,
                };

                let status_code = parts.next()?;
                parsed_response.status_code = StatusCode::try_from(status_code).ok()?;

                state = ParseState::Headers;
            }
            ParseState::Headers => {
                let mut parts = line.split(':');
                let key = parts.next()?.trim();
                let value = parts.next()?.trim();
                parsed_response.headers.insert(key, value);
            }
            ParseState::BodyStart => {
                parsed_response.body = String::from(line);
                state = ParseState::Body;
            }
            ParseState::Body => {
                parsed_response.body.push('\n');
                parsed_response.body.push_str(line);
            }
        }
    }

    Some(parsed_response)
}
