use std::{collections::HashMap, str::FromStr, time::Duration};

use async_trait::async_trait;
use http::{
    header::{CONTENT_LENGTH, CONTENT_TYPE, HOST},
    HeaderName, HeaderValue, StatusCode, Uri,
};
use hyper::{Body, Request, Response};
use regex::Regex;
use tokio::fs;
use tokio::time::sleep;

use crate::{
    core_api::rules::{
        FaultKind, HeaderMutation, RuleAction, RuleMatcher, RulePhase, TextMatcher,
        TypedRuleDefinition,
    },
    processors::{parser::ProcessorRuleParser, Processor},
};

use super::{
    HttpRequestProcessor, HttpResponseProcessor, ProcessorID, RequestProcessResult,
    ResponseProcessResult,
};

impl ProcessorID {
    pub const TYPED_RULES: ProcessorID = ProcessorID("TypedRules");
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TypedRuleProcessor {
    rules: Vec<CompiledTypedRule>,
}

#[derive(Debug, Clone)]
struct CompiledTypedRule {
    id: String,
    priority: i32,
    phase: RulePhase,
    matchers: Vec<CompiledMatcher>,
    actions: Vec<RuleAction>,
}

#[derive(Debug, Clone)]
enum CompiledMatcher {
    Method(String),
    Scheme(String),
    Host(CompiledTextMatcher),
    Port(u16),
    Path(CompiledTextMatcher),
    Query {
        name: Option<String>,
        matcher: CompiledTextMatcher,
    },
    Header {
        name: HeaderName,
        matcher: CompiledTextMatcher,
    },
    Status(u16),
    ContentType(CompiledTextMatcher),
    BodyPreview,
}

#[derive(Debug, Clone)]
enum CompiledTextMatcher {
    Exact(String),
    Contains(String),
    Regex(Regex),
}

impl TypedRuleProcessor {
    pub(crate) fn from_rules(rules: Vec<TypedRuleDefinition>) -> Result<Self, String> {
        let mut rules = rules
            .into_iter()
            .filter(|rule| rule.enabled)
            .map(CompiledTypedRule::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        rules.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(Self { rules })
    }
}

impl Processor for TypedRuleProcessor {
    fn name(&self) -> ProcessorID {
        ProcessorID::TYPED_RULES
    }
}

#[async_trait]
impl HttpRequestProcessor for TypedRuleProcessor {
    async fn process_request(&self, req: Request<Body>) -> RequestProcessResult {
        for rule in self.rules.iter() {
            if !rule.matches_request(&req) {
                continue;
            }

            return execute_request_rule(rule, req).await;
        }

        (req.into(), false, None)
    }
}

#[async_trait]
impl HttpResponseProcessor for TypedRuleProcessor {
    async fn process_response(&self, req_uri: &Uri, res: Response<Body>) -> ResponseProcessResult {
        for rule in self.rules.iter() {
            if !rule.matches_response(req_uri, &res) {
                continue;
            }

            return execute_response_rule(rule, res).await;
        }

        (res, false, None)
    }
}

impl ProcessorRuleParser for TypedRuleProcessor {
    type Rule = Vec<TypedRuleDefinition>;

    fn parse_rule(content: &str) -> Self::Rule {
        crate::core_api::rules::parse_typed_rules_dsl(content).unwrap_or_default()
    }
}

impl From<String> for TypedRuleProcessor {
    fn from(content: String) -> Self {
        Self::from_rules(Self::parse_rule(content.as_str())).unwrap_or_default()
    }
}

impl TryFrom<TypedRuleDefinition> for CompiledTypedRule {
    type Error = String;

    fn try_from(rule: TypedRuleDefinition) -> Result<Self, Self::Error> {
        Ok(Self {
            id: rule.id,
            priority: rule.priority,
            phase: rule.phase,
            matchers: rule
                .matchers
                .into_iter()
                .map(CompiledMatcher::try_from_rule_matcher)
                .collect::<Result<Vec<_>, _>>()?,
            actions: rule.actions,
        })
    }
}

impl CompiledTypedRule {
    fn matches_request(&self, req: &Request<Body>) -> bool {
        if self.phase != RulePhase::Request {
            return false;
        }

        self.matchers
            .iter()
            .all(|matcher| matcher.matches_request(req))
    }

    fn matches_response(&self, req_uri: &Uri, res: &Response<Body>) -> bool {
        if self.phase != RulePhase::Response {
            return false;
        }

        self.matchers
            .iter()
            .all(|matcher| matcher.matches_response(req_uri, res))
    }
}

impl CompiledMatcher {
    fn try_from_rule_matcher(matcher: RuleMatcher) -> Result<Self, String> {
        match matcher {
            RuleMatcher::Method { value } => Ok(Self::Method(value)),
            RuleMatcher::Scheme { value } => Ok(Self::Scheme(value)),
            RuleMatcher::Host { matcher } => {
                CompiledTextMatcher::try_from(matcher).map(Self::Host)
            }
            RuleMatcher::Port { value } => Ok(Self::Port(value)),
            RuleMatcher::Path { matcher } => {
                CompiledTextMatcher::try_from(matcher).map(Self::Path)
            }
            RuleMatcher::Query { name, matcher } => {
                CompiledTextMatcher::try_from(matcher).map(|matcher| Self::Query {
                    name,
                    matcher,
                })
            }
            RuleMatcher::Header { name, matcher } => {
                let name = HeaderName::from_str(name.as_str())
                    .map_err(|err| format!("Invalid typed rule header matcher({name}): {err}"));
                match name {
                    Ok(name) => CompiledTextMatcher::try_from(matcher)
                        .map(|matcher| Self::Header { name, matcher }),
                    Err(err) => Err(err),
                }
            }
            RuleMatcher::Status { value } => Ok(Self::Status(value)),
            RuleMatcher::ContentType { matcher } => {
                CompiledTextMatcher::try_from(matcher).map(Self::ContentType)
            }
            RuleMatcher::BodyPreview { matcher } => {
                CompiledTextMatcher::try_from(matcher).map(|_| Self::BodyPreview)
            }
        }
    }

    fn matches_request(&self, req: &Request<Body>) -> bool {
        match self {
            Self::Method(value) => req.method().as_str() == value,
            Self::Scheme(value) => req.uri().scheme_str() == Some(value.as_str()),
            Self::Host(matcher) => request_host(req).is_some_and(|host| matcher.matches(host)),
            Self::Port(value) => request_port(req) == Some(*value),
            Self::Path(matcher) => matcher.matches(req.uri().path()),
            Self::Query { name, matcher } => match name {
                Some(name) => request_query_value(req.uri(), name)
                    .is_some_and(|value| matcher.matches(value)),
                None => matcher.matches(req.uri().query().unwrap_or_default()),
            },
            Self::Header { name, matcher } => req
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| matcher.matches(value)),
            Self::Status(_) => false,
            Self::ContentType(matcher) => req
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| matcher.matches(value)),
            Self::BodyPreview => false,
        }
    }

    fn matches_response(&self, req_uri: &Uri, res: &Response<Body>) -> bool {
        match self {
            Self::Method(_) => false,
            Self::Scheme(value) => req_uri.scheme_str() == Some(value.as_str()),
            Self::Host(matcher) => req_uri.host().is_some_and(|host| matcher.matches(host)),
            Self::Port(value) => req_uri.port_u16() == Some(*value),
            Self::Path(matcher) => matcher.matches(req_uri.path()),
            Self::Query { name, matcher } => match name {
                Some(name) => request_query_value(req_uri, name)
                    .is_some_and(|value| matcher.matches(value)),
                None => matcher.matches(req_uri.query().unwrap_or_default()),
            },
            Self::Header { name, matcher } => res
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| matcher.matches(value)),
            Self::Status(value) => res.status().as_u16() == *value,
            Self::ContentType(matcher) => res
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| matcher.matches(value)),
            Self::BodyPreview => false,
        }
    }
}

impl CompiledTextMatcher {
    fn matches(&self, value: &str) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::Contains(expected) => value.contains(expected),
            Self::Regex(regex) => regex.is_match(value),
        }
    }
}

impl TryFrom<TextMatcher> for CompiledTextMatcher {
    type Error = String;

    fn try_from(matcher: TextMatcher) -> Result<Self, Self::Error> {
        match matcher {
            TextMatcher::Exact { value } => Ok(Self::Exact(value)),
            TextMatcher::Contains { value } => Ok(Self::Contains(value)),
            TextMatcher::Regex { pattern } => Regex::new(pattern.as_str())
                .map(Self::Regex)
                .map_err(|err| format!("Invalid typed rule regex({pattern}): {err}")),
        }
    }
}

async fn execute_request_rule(
    rule: &CompiledTypedRule,
    mut req: Request<Body>,
) -> RequestProcessResult {
    let mut hit_info = HashMap::<String, String>::new();
    hit_info.insert("ruleId".to_string(), rule.id.clone());
    let mut applied_actions = Vec::<&'static str>::new();

    for action in rule.actions.iter() {
        match action {
            RuleAction::Redirect { uri } | RuleAction::MapRemote { uri } => {
                let uri = match Uri::from_str(uri.as_str()) {
                    Ok(uri) => uri,
                    Err(err) => {
                        log::debug!("invalid typed redirect destination URI({uri}): {err}");
                        continue;
                    }
                };
                hit_info.insert("uri".to_string(), uri.to_string());
                *req.uri_mut() = uri;
                record_hit_action(&mut hit_info, &mut applied_actions, action);
            }
            RuleAction::Delay { millis } => {
                sleep(Duration::from_millis(*millis)).await;
                hit_info.insert("millis".to_string(), millis.to_string());
                record_hit_action(&mut hit_info, &mut applied_actions, action);
            }
            RuleAction::RequestHeader {
                operation,
                name,
                value,
            } => match apply_request_header_action(&mut req, *operation, name, value.as_deref()) {
                Ok(()) => {
                    hit_info.insert("header".to_string(), name.clone());
                    if let Some(value) = value {
                        hit_info.insert("value".to_string(), value.clone());
                    }
                    record_hit_action(&mut hit_info, &mut applied_actions, action);
                }
                Err(err) => {
                    log::debug!("{err}");
                }
            },
            RuleAction::RequestBody { body } => {
                let body_bytes = body.len();
                req = replace_request_body(req, body.clone());
                hit_info.insert("bodyBytes".to_string(), body_bytes.to_string());
                record_hit_action(&mut hit_info, &mut applied_actions, action);
            }
            RuleAction::MapLocal { path } => match fs::read(path).await {
                Ok(body) => {
                    let body_bytes = body.len();
                    let res = Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(body))
                        .expect("static status should build response");
                    hit_info.insert("path".to_string(), path.clone());
                    hit_info.insert("bodyBytes".to_string(), body_bytes.to_string());
                    record_hit_action(&mut hit_info, &mut applied_actions, action);
                    finish_hit_info(&mut hit_info, &applied_actions);
                    return ((req, res).into(), true, Some(hit_info));
                }
                Err(err) => {
                    log::debug!("typed map-local read failed({path}): {err}");
                }
            },
            RuleAction::Block { status, body } => match build_response_with_body(
                *status,
                Body::from(body.clone().unwrap_or_default()),
            ) {
                Ok(res) => {
                    hit_info.insert("status".to_string(), status.to_string());
                    record_hit_action(&mut hit_info, &mut applied_actions, action);
                    finish_hit_info(&mut hit_info, &applied_actions);
                    return ((req, res).into(), true, Some(hit_info));
                }
                Err(err) => {
                    log::debug!("{err}");
                }
            },
            RuleAction::Fault {
                fault:
                    FaultKind::AbortConnection | FaultKind::Timeout | FaultKind::InvalidResponse,
            }
            | RuleAction::ResponseHeader { .. }
            | RuleAction::ResponseBody { .. } => {}
        }
    }

    if applied_actions.is_empty() {
        (req.into(), false, None)
    } else {
        finish_hit_info(&mut hit_info, &applied_actions);
        (req.into(), true, Some(hit_info))
    }
}

async fn execute_response_rule(
    rule: &CompiledTypedRule,
    mut res: Response<Body>,
) -> ResponseProcessResult {
    let mut hit_info = HashMap::<String, String>::new();
    hit_info.insert("ruleId".to_string(), rule.id.clone());
    let mut applied_actions = Vec::<&'static str>::new();

    for action in rule.actions.iter() {
        match action {
            RuleAction::ResponseHeader {
                operation,
                name,
                value,
            } => match apply_response_header_action(&mut res, *operation, name, value.as_deref()) {
                Ok(()) => {
                    hit_info.insert("header".to_string(), name.clone());
                    if let Some(value) = value {
                        hit_info.insert("value".to_string(), value.clone());
                    }
                    record_hit_action(&mut hit_info, &mut applied_actions, action);
                }
                Err(err) => {
                    log::debug!("{err}");
                }
            },
            RuleAction::Delay { millis } => {
                sleep(Duration::from_millis(*millis)).await;
                hit_info.insert("millis".to_string(), millis.to_string());
                record_hit_action(&mut hit_info, &mut applied_actions, action);
            }
            RuleAction::ResponseBody { body } => {
                let body_bytes = body.len();
                res = replace_response_body(res, body.clone());
                hit_info.insert("bodyBytes".to_string(), body_bytes.to_string());
                record_hit_action(&mut hit_info, &mut applied_actions, action);
            }
            RuleAction::Block { status, body } => match replace_response_status_body(
                res,
                *status,
                body.clone().unwrap_or_default(),
            ) {
                Ok(blocked) => {
                    res = blocked;
                    hit_info.insert("status".to_string(), status.to_string());
                    record_hit_action(&mut hit_info, &mut applied_actions, action);
                }
                Err((original_res, err)) => {
                    res = original_res;
                    log::debug!("{err}");
                }
            },
            RuleAction::Fault { .. }
            | RuleAction::Redirect { .. }
            | RuleAction::MapRemote { .. }
            | RuleAction::MapLocal { .. }
            | RuleAction::RequestHeader { .. }
            | RuleAction::RequestBody { .. } => {}
        }
    }

    if applied_actions.is_empty() {
        (res, false, None)
    } else {
        finish_hit_info(&mut hit_info, &applied_actions);
        (res, true, Some(hit_info))
    }
}

fn apply_request_header_action(
    req: &mut Request<Body>,
    operation: HeaderMutation,
    name: &str,
    value: Option<&str>,
) -> Result<(), String> {
    let name = HeaderName::from_str(name)
        .map_err(|err| format!("Invalid typed request header name({name}): {err}"))?;
    match operation {
        HeaderMutation::Set => {
            let value = value.ok_or_else(|| {
                format!("Typed request header set action missing value for header {name}")
            })?;
            let value = HeaderValue::from_str(value)
                .map_err(|err| format!("Invalid typed request header value({value}): {err}"))?;
            req.headers_mut().insert(name, value);
        }
        HeaderMutation::Remove => {
            req.headers_mut().remove(name);
        }
    }
    Ok(())
}

fn apply_response_header_action(
    res: &mut Response<Body>,
    operation: HeaderMutation,
    name: &str,
    value: Option<&str>,
) -> Result<(), String> {
    let name = HeaderName::from_str(name)
        .map_err(|err| format!("Invalid typed response header name({name}): {err}"))?;
    match operation {
        HeaderMutation::Set => {
            let value = value.ok_or_else(|| {
                format!("Typed response header set action missing value for header {name}")
            })?;
            let value = HeaderValue::from_str(value)
                .map_err(|err| format!("Invalid typed response header value({value}): {err}"))?;
            res.headers_mut().insert(name, value);
        }
        HeaderMutation::Remove => {
            res.headers_mut().remove(name);
        }
    }
    Ok(())
}

fn replace_request_body(req: Request<Body>, body: String) -> Request<Body> {
    let (mut parts, _) = req.into_parts();
    parts.headers.remove(CONTENT_LENGTH);
    Request::from_parts(parts, Body::from(body))
}

fn replace_response_body(res: Response<Body>, body: String) -> Response<Body> {
    let (mut parts, _) = res.into_parts();
    parts.headers.remove(CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(body))
}

fn replace_response_status_body(
    res: Response<Body>,
    status: u16,
    body: String,
) -> Result<Response<Body>, (Response<Body>, String)> {
    let status_code = match StatusCode::from_u16(status) {
        Ok(status_code) => status_code,
        Err(err) => {
            return Err((
                res,
                format!("Invalid typed response block status({status}): {err}"),
            ));
        }
    };
    let (mut parts, _) = res.into_parts();
    parts.status = status_code;
    parts.headers.remove(CONTENT_LENGTH);
    Ok(Response::from_parts(parts, Body::from(body)))
}

fn build_response_with_body(status: u16, body: Body) -> Result<Response<Body>, String> {
    Response::builder()
        .status(status)
        .body(body)
        .map_err(|err| format!("Invalid typed block response status({status}): {err}"))
}

fn record_hit_action(
    hit_info: &mut HashMap<String, String>,
    applied_actions: &mut Vec<&'static str>,
    action: &RuleAction,
) {
    let kind = action.kind_name();
    if applied_actions.is_empty() {
        hit_info.insert("action".to_string(), kind.to_string());
    }
    applied_actions.push(kind);
}

fn finish_hit_info(
    hit_info: &mut HashMap<String, String>,
    applied_actions: &[&'static str],
) {
    if applied_actions.len() > 1 {
        hit_info.insert("actions".to_string(), applied_actions.join(","));
    }
}

fn request_host(req: &Request<Body>) -> Option<&str> {
    req.uri()
        .host()
        .or_else(|| req.headers().get(HOST).and_then(|value| value.to_str().ok()))
}

fn request_port(req: &Request<Body>) -> Option<u16> {
    req.uri().port_u16()
}

fn request_query_value<'a>(uri: &'a Uri, name: &str) -> Option<&'a str> {
    uri.query()?.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        (key == name).then_some(value)
    })
}

trait RuleActionName {
    fn kind_name(&self) -> &'static str;
}

impl RuleActionName for RuleAction {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Redirect { .. } => "redirect",
            Self::Delay { .. } => "delay",
            Self::MapLocal { .. } => "mapLocal",
            Self::MapRemote { .. } => "mapRemote",
            Self::Block { .. } => "block",
            Self::RequestHeader { .. } => "requestHeader",
            Self::ResponseHeader { .. } => "responseHeader",
            Self::RequestBody { .. } => "requestBody",
            Self::ResponseBody { .. } => "responseBody",
            Self::Fault { .. } => "fault",
        }
    }
}
