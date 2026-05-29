use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::core_api::{ensure_supported_version, CORE_API_VERSION};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum RuleKind {
    Delay,
    Redirect,
    RequestHeader,
    Response,
    ResponseHeader,
}

impl RuleKind {
    pub(crate) fn legacy_processor_mode(self) -> &'static str {
        match self {
            Self::Delay => "Delay",
            Self::Redirect => "Redirect",
            Self::RequestHeader => "RequestHeader",
            Self::Response => "Response",
            Self::ResponseHeader => "ResponseHeader",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetRuleContentRequest {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
    pub(crate) kind: RuleKind,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetRuleContentResponse {
    pub(crate) api_version: u16,
    pub(crate) saved: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetRuleContentRequest {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
    pub(crate) kind: RuleKind,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetRuleContentResponse {
    pub(crate) api_version: u16,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListRulePacksRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListRulePacksResponse {
    pub(crate) api_version: u16,
    pub(crate) packs: Vec<RulePackSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RulePackSummary {
    pub(crate) pack_name: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddRulePackRequest {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoveRulePackRequest {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateRulePackStatusRequest {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RulePackMutationResponse {
    pub(crate) api_version: u16,
    pub(crate) changed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveTypedRulesRequest {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
    pub(crate) rules: Vec<TypedRuleDefinition>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveRulePackRulesRequest {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
    pub(crate) enabled: bool,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetRulePackRulesRequest {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RulePackRulesResponse {
    pub(crate) api_version: u16,
    pub(crate) pack_name: String,
    pub(crate) enabled: bool,
    pub(crate) content: String,
    pub(crate) rules: Vec<TypedRuleDefinition>,
    pub(crate) evaluation_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ValidateTypedRulesResponse {
    pub(crate) api_version: u16,
    pub(crate) valid: bool,
    pub(crate) evaluation_order: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TypedRuleDefinition {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) enabled: bool,
    pub(crate) priority: i32,
    pub(crate) phase: RulePhase,
    pub(crate) matchers: Vec<RuleMatcher>,
    pub(crate) actions: Vec<RuleAction>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RulePhase {
    Request,
    Response,
}

impl RulePhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Response => "response",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "operator", rename_all = "camelCase")]
pub(crate) enum TextMatcher {
    Exact { value: String },
    Contains { value: String },
    Regex { pattern: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum RuleMatcher {
    Method { value: String },
    Scheme { value: String },
    Host {
        #[serde(flatten)]
        matcher: TextMatcher,
    },
    Port { value: u16 },
    Path {
        #[serde(flatten)]
        matcher: TextMatcher,
    },
    Query {
        name: Option<String>,
        #[serde(flatten)]
        matcher: TextMatcher,
    },
    Header {
        name: String,
        #[serde(flatten)]
        matcher: TextMatcher,
    },
    Status { value: u16 },
    ContentType {
        #[serde(flatten)]
        matcher: TextMatcher,
    },
    BodyPreview {
        #[serde(flatten)]
        matcher: TextMatcher,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum HeaderMutation {
    Set,
    Remove,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum RuleAction {
    Redirect { uri: String },
    Delay { millis: u64 },
    MapLocal { path: String },
    MapRemote { uri: String },
    Block { status: u16, body: Option<String> },
    RequestHeader {
        operation: HeaderMutation,
        name: String,
        value: Option<String>,
    },
    ResponseHeader {
        operation: HeaderMutation,
        name: String,
        value: Option<String>,
    },
    RequestBody { body: String },
    ResponseBody { body: String },
    Fault { fault: FaultKind },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum FaultKind {
    AbortConnection,
    Timeout,
    InvalidResponse,
}

pub(crate) fn validate_set_rule_content_request(
    request: &SetRuleContentRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;
    validate_pack_name(request.pack_name.as_str())
}

pub(crate) fn validate_get_rule_content_request(
    request: &GetRuleContentRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;
    validate_pack_name(request.pack_name.as_str())
}

pub(crate) fn validate_list_rule_packs_request(
    request: &ListRulePacksRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)
}

pub(crate) fn validate_add_rule_pack_request(
    request: &AddRulePackRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;
    validate_pack_name(request.pack_name.as_str())
}

pub(crate) fn validate_remove_rule_pack_request(
    request: &RemoveRulePackRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;
    validate_pack_name(request.pack_name.as_str())
}

pub(crate) fn validate_update_rule_pack_status_request(
    request: &UpdateRulePackStatusRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;
    validate_pack_name(request.pack_name.as_str())
}

pub(crate) fn validate_save_typed_rules_request(
    request: &SaveTypedRulesRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;
    validate_pack_name(request.pack_name.as_str())?;

    let mut rule_ids = HashSet::<String>::new();
    for rule in request.rules.iter() {
        validate_typed_rule_definition(rule)?;
        if !rule_ids.insert(rule.id.clone()) {
            return Err(format!("Duplicate rule id: {}", rule.id));
        }
    }

    Ok(())
}

pub(crate) fn parse_save_rule_pack_rules_request(
    request: &SaveRulePackRulesRequest,
) -> Result<Vec<TypedRuleDefinition>, String> {
    ensure_supported_version(request.api_version)?;
    validate_pack_name(request.pack_name.as_str())?;
    parse_rule_pack_rules_content(request.pack_name.as_str(), request.content.as_str())
}

pub(crate) fn validate_get_rule_pack_rules_request(
    request: &GetRulePackRulesRequest,
) -> Result<(), String> {
    ensure_supported_version(request.api_version)?;
    validate_pack_name(request.pack_name.as_str())
}

pub(crate) fn set_rule_content_response(saved: bool) -> SetRuleContentResponse {
    SetRuleContentResponse {
        api_version: CORE_API_VERSION,
        saved,
    }
}

pub(crate) fn get_rule_content_response(content: String) -> GetRuleContentResponse {
    GetRuleContentResponse {
        api_version: CORE_API_VERSION,
        content,
    }
}

pub(crate) fn list_rule_packs_response(
    packs: Vec<RulePackSummary>,
) -> ListRulePacksResponse {
    ListRulePacksResponse {
        api_version: CORE_API_VERSION,
        packs,
    }
}

pub(crate) fn rule_pack_mutation_response(changed: bool) -> RulePackMutationResponse {
    RulePackMutationResponse {
        api_version: CORE_API_VERSION,
        changed,
    }
}

pub(crate) fn rule_pack_rules_response(
    pack_name: String,
    enabled: bool,
    content: String,
) -> Result<RulePackRulesResponse, String> {
    let rules = parse_rule_pack_rules_content(pack_name.as_str(), content.as_str())?;
    let evaluation_order = typed_rule_evaluation_order(&rules);

    Ok(RulePackRulesResponse {
        api_version: CORE_API_VERSION,
        pack_name,
        enabled,
        content,
        rules,
        evaluation_order,
    })
}

pub(crate) fn validate_typed_rules_response(
    rules: &[TypedRuleDefinition],
) -> ValidateTypedRulesResponse {
    ValidateTypedRulesResponse {
        api_version: CORE_API_VERSION,
        valid: true,
        evaluation_order: typed_rule_evaluation_order(rules),
    }
}

pub(crate) fn typed_rule_evaluation_order(
    rules: &[TypedRuleDefinition],
) -> Vec<String> {
    let mut rules = rules.iter().collect::<Vec<_>>();
    rules.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.id.cmp(&right.id))
    });
    rules.into_iter().map(|rule| rule.id.clone()).collect()
}

fn parse_rule_pack_rules_content(
    pack_name: &str,
    content: &str,
) -> Result<Vec<TypedRuleDefinition>, String> {
    let rules = parse_typed_rules_dsl(content)?;
    validate_save_typed_rules_request(&SaveTypedRulesRequest {
        api_version: CORE_API_VERSION,
        pack_name: pack_name.to_string(),
        rules: rules.clone(),
    })?;
    Ok(rules)
}

const DSL_BASE_PRIORITY: i32 = 1_000_000;

pub(crate) fn parse_typed_rules_dsl(input: &str) -> Result<Vec<TypedRuleDefinition>, String> {
    let mut rules = Vec::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line
            .split('#')
            .next()
            .unwrap_or_default()
            .trim();

        if line.is_empty() {
            continue;
        }

        rules.push(parse_typed_rules_dsl_line(
            line,
            line_number,
            rules.len(),
        )?);
    }

    Ok(rules)
}

pub(crate) fn format_typed_rules_dsl(rules: &[TypedRuleDefinition]) -> Result<String, String> {
    let mut ordered_rules = rules.iter().collect::<Vec<_>>();
    ordered_rules.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut output = String::new();
    for rule in ordered_rules {
        output.push_str(format_typed_rule_dsl_line(rule)?.as_str());
        output.push('\n');
    }

    Ok(output)
}

fn parse_typed_rules_dsl_line(
    line: &str,
    line_number: usize,
    rule_index: usize,
) -> Result<TypedRuleDefinition, String> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let normalized_line = tokens.join(" ");
    let action = tokens
        .first()
        .ok_or_else(|| dsl_line_error(line_number, "rule line must not be empty"))?;

    let (phase, matchers, actions) = match *action {
        "redirect" => {
            expect_dsl_columns(line_number, &tokens, 4, "redirect METHOD URL_GLOB DESTINATION")?;
            (
                RulePhase::Request,
                parse_request_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::Redirect {
                    uri: tokens[3].to_string(),
                }],
            )
        }
        "map-remote" => {
            expect_dsl_columns(line_number, &tokens, 4, "map-remote METHOD URL_GLOB DESTINATION")?;
            (
                RulePhase::Request,
                parse_request_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::MapRemote {
                    uri: tokens[3].to_string(),
                }],
            )
        }
        "map-local" => {
            expect_dsl_min_columns(line_number, &tokens, 4, "map-local METHOD URL_GLOB FILE_PATH")?;
            (
                RulePhase::Request,
                parse_request_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::MapLocal {
                    path: tokens[3..].join(" "),
                }],
            )
        }
        "delay" => {
            expect_dsl_columns(line_number, &tokens, 4, "delay METHOD URL_GLOB 500ms")?;
            let millis = parse_dsl_duration_millis(tokens[3]).ok_or_else(|| {
                dsl_line_error(line_number, "delay expects a duration like 500ms")
            })?;
            (
                RulePhase::Request,
                parse_request_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::Delay { millis }],
            )
        }
        "request-header" => {
            expect_dsl_min_columns(
                line_number,
                &tokens,
                5,
                "request-header METHOD URL_GLOB HEADER_NAME VALUE",
            )?;
            (
                RulePhase::Request,
                parse_request_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::RequestHeader {
                    operation: HeaderMutation::Set,
                    name: tokens[3].to_string(),
                    value: Some(tokens[4..].join(" ")),
                }],
            )
        }
        "response-header" => {
            expect_dsl_min_columns(
                line_number,
                &tokens,
                5,
                "response-header STATUS URL_GLOB HEADER_NAME VALUE",
            )?;
            (
                RulePhase::Response,
                parse_response_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::ResponseHeader {
                    operation: HeaderMutation::Set,
                    name: tokens[3].to_string(),
                    value: Some(tokens[4..].join(" ")),
                }],
            )
        }
        "request-body" => {
            expect_dsl_min_columns(
                line_number,
                &tokens,
                4,
                "request-body METHOD URL_GLOB BODY",
            )?;
            (
                RulePhase::Request,
                parse_request_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::RequestBody {
                    body: tokens[3..].join(" "),
                }],
            )
        }
        "response-body" => {
            expect_dsl_min_columns(
                line_number,
                &tokens,
                4,
                "response-body STATUS URL_GLOB BODY",
            )?;
            (
                RulePhase::Response,
                parse_response_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::ResponseBody {
                    body: tokens[3..].join(" "),
                }],
            )
        }
        "block" => {
            expect_dsl_min_columns(line_number, &tokens, 4, "block METHOD URL_GLOB STATUS")?;
            let status = parse_dsl_status(tokens[3], line_number)?;
            (
                RulePhase::Request,
                parse_request_dsl_matchers(tokens[1], tokens[2], line_number)?,
                vec![RuleAction::Block {
                    status,
                    body: dsl_optional_tail(&tokens, 4),
                }],
            )
        }
        _ => {
            return Err(dsl_line_error(
                line_number,
                format!("unknown rule action: {action}").as_str(),
            ));
        }
    };

    let rule = TypedRuleDefinition {
        id: format!(
            "dsl-{}-{:016x}",
            rule_index + 1,
            stable_dsl_hash(normalized_line.as_str())
        ),
        name: None,
        enabled: true,
        priority: DSL_BASE_PRIORITY - rule_index as i32,
        phase,
        matchers,
        actions,
    };

    validate_typed_rule_definition(&rule).map_err(|err| dsl_line_error(line_number, &err))?;

    Ok(rule)
}

fn parse_request_dsl_matchers(
    method: &str,
    url_glob: &str,
    line_number: usize,
) -> Result<Vec<RuleMatcher>, String> {
    let mut matchers = Vec::new();
    if method != "*" {
        let value = method.to_ascii_uppercase();
        validate_method(value.as_str()).map_err(|err| dsl_line_error(line_number, &err))?;
        matchers.push(RuleMatcher::Method { value });
    }

    matchers.extend(parse_url_glob_matchers(url_glob, line_number)?);
    ensure_dsl_has_matcher(&mut matchers);
    Ok(matchers)
}

fn parse_response_dsl_matchers(
    status: &str,
    url_glob: &str,
    line_number: usize,
) -> Result<Vec<RuleMatcher>, String> {
    let mut matchers = Vec::new();
    if status != "*" {
        matchers.push(RuleMatcher::Status {
            value: parse_dsl_status(status, line_number)?,
        });
    }

    matchers.extend(parse_url_glob_matchers(url_glob, line_number)?);
    ensure_dsl_has_matcher(&mut matchers);
    Ok(matchers)
}

fn parse_url_glob_matchers(
    url_glob: &str,
    line_number: usize,
) -> Result<Vec<RuleMatcher>, String> {
    if url_glob.trim().is_empty() {
        return Err(dsl_line_error(line_number, "url glob must not be empty"));
    }

    if url_glob == "*" {
        return Ok(Vec::new());
    }

    let mut matchers = Vec::new();
    let mut remaining = url_glob;

    if let Some((scheme, rest)) = url_glob.split_once("://") {
        if !scheme.is_empty() && scheme != "*" {
            validate_scheme(scheme).map_err(|err| dsl_line_error(line_number, &err))?;
            matchers.push(RuleMatcher::Scheme {
                value: scheme.to_string(),
            });
        }
        remaining = rest;
    }

    let (host_glob, path_glob) = if let Some(path_start) = remaining.find('/') {
        (&remaining[..path_start], &remaining[path_start..])
    } else {
        (remaining, "")
    };

    let (host_glob, port) = split_host_port_glob(host_glob);

    if !host_glob.is_empty() && host_glob != "*" {
        matchers.push(RuleMatcher::Host {
            matcher: TextMatcher::Regex {
                pattern: glob_to_regex(host_glob),
            },
        });
    }

    if let Some(port) = port {
        matchers.push(RuleMatcher::Port { value: port });
    }

    if !path_glob.is_empty() && path_glob != "*" {
        matchers.push(RuleMatcher::Path {
            matcher: TextMatcher::Regex {
                pattern: glob_to_regex(path_glob),
            },
        });
    }

    Ok(matchers)
}

fn split_host_port_glob(host_glob: &str) -> (&str, Option<u16>) {
    if let Some((host, port)) = host_glob.rsplit_once(':') {
        if !host.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) {
            if let Ok(port) = port.parse::<u16>() {
                return (host, Some(port));
            }
        }
    }

    (host_glob, None)
}

fn ensure_dsl_has_matcher(matchers: &mut Vec<RuleMatcher>) {
    if matchers.is_empty() {
        matchers.push(RuleMatcher::Host {
            matcher: TextMatcher::Regex {
                pattern: "^.*$".to_string(),
            },
        });
    }
}

fn format_typed_rule_dsl_line(rule: &TypedRuleDefinition) -> Result<String, String> {
    if !rule.enabled {
        return Err(format!(
            "Rule {} cannot be formatted as simple DSL because it is disabled",
            rule.id
        ));
    }

    let action = rule.actions.first().ok_or_else(|| {
        format!(
            "Rule {} cannot be formatted as simple DSL because it has no actions",
            rule.id
        )
    })?;
    if rule.actions.len() != 1 {
        return Err(format!(
            "Rule {} cannot be formatted as simple DSL because it has multiple actions",
            rule.id
        ));
    }

    let url_glob = rule_url_glob(rule)?;
    match action {
        RuleAction::Redirect { uri } => {
            ensure_dsl_phase(rule, RulePhase::Request)?;
            Ok(format!(
                "redirect {} {} {}",
                request_dsl_selector(rule),
                url_glob,
                uri
            ))
        }
        RuleAction::MapRemote { uri } => {
            ensure_dsl_phase(rule, RulePhase::Request)?;
            Ok(format!(
                "map-remote {} {} {}",
                request_dsl_selector(rule),
                url_glob,
                uri
            ))
        }
        RuleAction::MapLocal { path } => {
            ensure_dsl_phase(rule, RulePhase::Request)?;
            Ok(format!(
                "map-local {} {} {}",
                request_dsl_selector(rule),
                url_glob,
                path
            ))
        }
        RuleAction::Delay { millis } => {
            ensure_dsl_phase(rule, RulePhase::Request)?;
            Ok(format!(
                "delay {} {} {}ms",
                request_dsl_selector(rule),
                url_glob,
                millis
            ))
        }
        RuleAction::RequestHeader {
            operation,
            name,
            value,
        } => {
            ensure_dsl_phase(rule, RulePhase::Request)?;
            ensure_dsl_header_set(rule, *operation, value.as_deref())?;
            Ok(format!(
                "request-header {} {} {} {}",
                request_dsl_selector(rule),
                url_glob,
                name,
                value.as_deref().unwrap_or_default()
            ))
        }
        RuleAction::ResponseHeader {
            operation,
            name,
            value,
        } => {
            ensure_dsl_phase(rule, RulePhase::Response)?;
            ensure_dsl_header_set(rule, *operation, value.as_deref())?;
            Ok(format!(
                "response-header {} {} {} {}",
                response_dsl_selector(rule),
                url_glob,
                name,
                value.as_deref().unwrap_or_default()
            ))
        }
        RuleAction::RequestBody { body } => {
            ensure_dsl_phase(rule, RulePhase::Request)?;
            Ok(format!(
                "request-body {} {} {}",
                request_dsl_selector(rule),
                url_glob,
                body
            ))
        }
        RuleAction::ResponseBody { body } => {
            ensure_dsl_phase(rule, RulePhase::Response)?;
            Ok(format!(
                "response-body {} {} {}",
                response_dsl_selector(rule),
                url_glob,
                body
            ))
        }
        RuleAction::Block { status, body } => {
            ensure_dsl_phase(rule, RulePhase::Request)?;
            if let Some(body) = body {
                Ok(format!(
                    "block {} {} {} {}",
                    request_dsl_selector(rule),
                    url_glob,
                    status,
                    body
                ))
            } else {
                Ok(format!(
                    "block {} {} {}",
                    request_dsl_selector(rule),
                    url_glob,
                    status
                ))
            }
        }
        _ => Err(format!(
            "Rule {} action {} is not supported by simple DSL formatting",
            rule.id,
            action.kind_name()
        )),
    }
}

fn request_dsl_selector(rule: &TypedRuleDefinition) -> String {
    rule.matchers
        .iter()
        .find_map(|matcher| match matcher {
            RuleMatcher::Method { value } => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "*".to_string())
}

fn response_dsl_selector(rule: &TypedRuleDefinition) -> String {
    rule.matchers
        .iter()
        .find_map(|matcher| match matcher {
            RuleMatcher::Status { value } => Some(value.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "*".to_string())
}

fn rule_url_glob(rule: &TypedRuleDefinition) -> Result<String, String> {
    let scheme = rule.matchers.iter().find_map(|matcher| match matcher {
        RuleMatcher::Scheme { value } => Some(value.as_str()),
        _ => None,
    });
    let host = rule
        .matchers
        .iter()
        .find_map(|matcher| match matcher {
            RuleMatcher::Host { matcher } => Some(text_matcher_to_glob(matcher)),
            _ => None,
        })
        .transpose()?
        .unwrap_or_else(|| "*".to_string());
    let port = rule.matchers.iter().find_map(|matcher| match matcher {
        RuleMatcher::Port { value } => Some(*value),
        _ => None,
    });
    let path = rule
        .matchers
        .iter()
        .find_map(|matcher| match matcher {
            RuleMatcher::Path { matcher } => Some(text_matcher_to_glob(matcher)),
            _ => None,
        })
        .transpose()?;

    if scheme.is_none() && host == "*" && path.is_none() {
        return Ok("*".to_string());
    }

    let authority = if let Some(port) = port {
        format!("{host}:{port}")
    } else {
        host
    };
    let path = path.unwrap_or_default();
    Ok(if let Some(scheme) = scheme {
        format!("{scheme}://{authority}{path}")
    } else {
        format!("{authority}{path}")
    })
}

fn text_matcher_to_glob(matcher: &TextMatcher) -> Result<String, String> {
    match matcher {
        TextMatcher::Exact { value } => Ok(value.clone()),
        TextMatcher::Contains { value } => Ok(format!("*{value}*")),
        TextMatcher::Regex { pattern } => regex_to_glob(pattern)
            .ok_or_else(|| format!("Regex matcher cannot be represented as a URL glob: {pattern}")),
    }
}

fn ensure_dsl_phase(rule: &TypedRuleDefinition, phase: RulePhase) -> Result<(), String> {
    if rule.phase == phase {
        Ok(())
    } else {
        Err(format!(
            "Rule {} cannot be formatted as simple DSL because it is a {} rule",
            rule.id,
            rule.phase.as_str()
        ))
    }
}

fn ensure_dsl_header_set(
    rule: &TypedRuleDefinition,
    operation: HeaderMutation,
    value: Option<&str>,
) -> Result<(), String> {
    if operation == HeaderMutation::Set && value.is_some_and(|value| !value.is_empty()) {
        Ok(())
    } else {
        Err(format!(
            "Rule {} cannot be formatted as simple DSL because only header set actions are supported",
            rule.id
        ))
    }
}

fn expect_dsl_columns(
    line_number: usize,
    tokens: &[&str],
    expected: usize,
    usage: &str,
) -> Result<(), String> {
    if tokens.len() == expected {
        Ok(())
    } else {
        Err(dsl_line_error(
            line_number,
            format!("expected {usage}").as_str(),
        ))
    }
}

fn expect_dsl_min_columns(
    line_number: usize,
    tokens: &[&str],
    expected: usize,
    usage: &str,
) -> Result<(), String> {
    if tokens.len() >= expected {
        Ok(())
    } else {
        Err(dsl_line_error(
            line_number,
            format!("expected {usage}").as_str(),
        ))
    }
}

fn parse_dsl_duration_millis(value: &str) -> Option<u64> {
    if let Some(millis) = value.strip_suffix("ms") {
        millis.parse::<u64>().ok()
    } else if let Some(seconds) = value.strip_suffix('s') {
        seconds.parse::<u64>().ok()?.checked_mul(1_000)
    } else {
        None
    }
}

fn parse_dsl_status(value: &str, line_number: usize) -> Result<u16, String> {
    let status = value.parse::<u16>().map_err(|_| {
        dsl_line_error(
            line_number,
            format!("status must be a number between 100 and 599: {value}").as_str(),
        )
    })?;
    validate_status("dsl", status).map_err(|err| dsl_line_error(line_number, &err))?;
    Ok(status)
}

fn dsl_optional_tail(tokens: &[&str], start: usize) -> Option<String> {
    if tokens.len() > start {
        Some(tokens[start..].join(" "))
    } else {
        None
    }
}

fn dsl_line_error(line_number: usize, message: &str) -> String {
    format!("Line {line_number}: {message}")
}

fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::from("^");
    for ch in glob.chars() {
        if ch == '*' {
            regex.push_str(".*");
        } else {
            push_regex_escaped(&mut regex, ch);
        }
    }
    regex.push('$');
    regex
}

fn regex_to_glob(pattern: &str) -> Option<String> {
    let inner = pattern.strip_prefix('^')?.strip_suffix('$')?;
    let mut glob = String::new();
    let mut chars = inner.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '.' && chars.peek().is_some_and(|next| *next == '*') {
            chars.next();
            glob.push('*');
        } else if ch == '\\' {
            glob.push(chars.next()?);
        } else if regex_special_char(ch) {
            return None;
        } else {
            glob.push(ch);
        }
    }

    Some(glob)
}

fn push_regex_escaped(output: &mut String, ch: char) {
    if regex_special_char(ch) {
        output.push('\\');
    }
    output.push(ch);
}

fn regex_special_char(ch: char) -> bool {
    matches!(
        ch,
        '.' | '+'
            | '?'
            | '('
            | ')'
            | '|'
            | '['
            | ']'
            | '{'
            | '}'
            | '^'
            | '$'
            | '\\'
    )
}

fn stable_dsl_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn validate_typed_rule_definition(rule: &TypedRuleDefinition) -> Result<(), String> {
    if rule.id.trim().is_empty() {
        return Err("Rule id must not be empty".to_string());
    }

    if rule.matchers.is_empty() {
        return Err(format!("Rule {} must include at least one matcher", rule.id));
    }

    if rule.actions.is_empty() {
        return Err(format!("Rule {} must include at least one action", rule.id));
    }

    for matcher in rule.matchers.iter() {
        validate_rule_matcher(rule.id.as_str(), rule.phase, matcher)?;
    }

    for action in rule.actions.iter() {
        validate_rule_action(rule.id.as_str(), rule.phase, action)?;
    }

    Ok(())
}

fn validate_rule_matcher(
    rule_id: &str,
    phase: RulePhase,
    matcher: &RuleMatcher,
) -> Result<(), String> {
    match matcher {
        RuleMatcher::Method { value } => validate_method(value),
        RuleMatcher::Scheme { value } => validate_scheme(value),
        RuleMatcher::Host { matcher }
        | RuleMatcher::Path { matcher }
        | RuleMatcher::ContentType { matcher }
        | RuleMatcher::BodyPreview { matcher } => validate_text_matcher(matcher),
        RuleMatcher::Port { value } => {
            if *value == 0 {
                Err(format!("Rule {rule_id} port matcher must not be 0"))
            } else {
                Ok(())
            }
        }
        RuleMatcher::Query { name, matcher } => {
            if name.as_deref().is_some_and(|name| name.trim().is_empty()) {
                return Err(format!("Rule {rule_id} query matcher name must not be empty"));
            }
            validate_text_matcher(matcher)
        }
        RuleMatcher::Header { name, matcher } => {
            validate_header_name(rule_id, name)?;
            validate_text_matcher(matcher)
        }
        RuleMatcher::Status { value } => {
            if phase != RulePhase::Response {
                return Err(format!(
                    "Rule {rule_id} matcher status is not valid for {} phase",
                    phase.as_str()
                ));
            }
            validate_status(rule_id, *value)
        }
    }
}

fn validate_rule_action(
    rule_id: &str,
    phase: RulePhase,
    action: &RuleAction,
) -> Result<(), String> {
    if !action_is_valid_for_phase(action, phase) {
        return Err(format!(
            "Rule {rule_id} action {} is not valid for {} phase",
            action.kind_name(),
            phase.as_str()
        ));
    }

    match action {
        RuleAction::Redirect { uri } | RuleAction::MapRemote { uri } => {
            validate_non_empty(rule_id, action.kind_name(), uri)
        }
        RuleAction::Delay { .. } | RuleAction::Fault { .. } => Ok(()),
        RuleAction::MapLocal { path } => validate_non_empty(rule_id, "mapLocal", path),
        RuleAction::Block { status, .. } => validate_status(rule_id, *status),
        RuleAction::RequestHeader {
            operation,
            name,
            value,
        }
        | RuleAction::ResponseHeader {
            operation,
            name,
            value,
        } => validate_header_mutation(rule_id, *operation, name, value.as_deref()),
        RuleAction::RequestBody { .. } | RuleAction::ResponseBody { .. } => Ok(()),
    }
}

fn action_is_valid_for_phase(action: &RuleAction, phase: RulePhase) -> bool {
    match phase {
        RulePhase::Request => matches!(
            action,
            RuleAction::Redirect { .. }
                | RuleAction::Delay { .. }
                | RuleAction::MapLocal { .. }
                | RuleAction::MapRemote { .. }
                | RuleAction::Block { .. }
                | RuleAction::RequestHeader { .. }
                | RuleAction::RequestBody { .. }
                | RuleAction::Fault { .. }
        ),
        RulePhase::Response => matches!(
            action,
            RuleAction::Delay { .. }
                | RuleAction::Block { .. }
                | RuleAction::ResponseHeader { .. }
                | RuleAction::ResponseBody { .. }
                | RuleAction::Fault { .. }
        ),
    }
}

impl RuleAction {
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

fn validate_text_matcher(matcher: &TextMatcher) -> Result<(), String> {
    match matcher {
        TextMatcher::Exact { value } | TextMatcher::Contains { value } => {
            if value.is_empty() {
                Err("Rule text matcher value must not be empty".to_string())
            } else {
                Ok(())
            }
        }
        TextMatcher::Regex { pattern } => regex::Regex::new(pattern)
            .map(|_| ())
            .map_err(|err| format!("Invalid rule matcher regex({pattern}): {err}")),
    }
}

fn validate_method(value: &str) -> Result<(), String> {
    value
        .parse::<http::Method>()
        .map(|_| ())
        .map_err(|err| format!("Invalid rule method matcher({value}): {err}"))
}

fn validate_scheme(value: &str) -> Result<(), String> {
    match value {
        "http" | "https" | "ws" | "wss" => Ok(()),
        _ => Err(format!("Invalid rule scheme matcher: {value}")),
    }
}

fn validate_header_mutation(
    rule_id: &str,
    operation: HeaderMutation,
    name: &str,
    value: Option<&str>,
) -> Result<(), String> {
    validate_header_name(rule_id, name)?;
    if operation == HeaderMutation::Set && value.is_none_or(str::is_empty) {
        return Err(format!(
            "Rule {rule_id} header set action must include a value"
        ));
    }
    Ok(())
}

fn validate_header_name(rule_id: &str, name: &str) -> Result<(), String> {
    name.parse::<http::HeaderName>()
        .map(|_| ())
        .map_err(|err| format!("Rule {rule_id} has invalid header name({name}): {err}"))
}

fn validate_status(rule_id: &str, status: u16) -> Result<(), String> {
    if (100..=599).contains(&status) {
        Ok(())
    } else {
        Err(format!("Rule {rule_id} has invalid status: {status}"))
    }
}

fn validate_non_empty(rule_id: &str, action: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("Rule {rule_id} action {action} value must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_pack_name(pack_name: &str) -> Result<(), String> {
    let pack_name = pack_name.trim();
    if pack_name.is_empty() {
        Err("Rule pack name must not be empty".to_string())
    } else if pack_name == "."
        || pack_name == ".."
        || pack_name.contains('/')
        || pack_name.contains('\\')
        || pack_name.contains('\0')
    {
        Err("Rule pack name must not contain path separators".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::CORE_API_VERSION;

    #[test]
    fn core_api_rules_rejects_unsupported_api_version() {
        let err = validate_set_rule_content_request(&SetRuleContentRequest {
            api_version: CORE_API_VERSION + 1,
            pack_name: "default".to_string(),
            kind: RuleKind::Redirect,
            content: "a b".to_string(),
        })
        .expect_err("unsupported version should fail");

        assert_eq!(
            err,
            format!("Unsupported core API version: {}", CORE_API_VERSION + 1)
        );
    }

    #[test]
    fn core_api_rules_rejects_empty_pack_name() {
        let err = validate_set_rule_content_request(&SetRuleContentRequest {
            api_version: CORE_API_VERSION,
            pack_name: "  ".to_string(),
            kind: RuleKind::Redirect,
            content: "a b".to_string(),
        })
        .expect_err("empty pack name should fail");

        assert_eq!(err, "Rule pack name must not be empty");
    }

    #[test]
    fn core_api_rules_rejects_unsafe_pack_name() {
        let err = validate_set_rule_content_request(&SetRuleContentRequest {
            api_version: CORE_API_VERSION,
            pack_name: "../default".to_string(),
            kind: RuleKind::Redirect,
            content: "a b".to_string(),
        })
        .expect_err("path-like pack name should fail");

        assert_eq!(err, "Rule pack name must not contain path separators");
    }

    #[test]
    fn core_api_rules_maps_typed_kind_to_legacy_processor_mode() {
        assert_eq!(RuleKind::Delay.legacy_processor_mode(), "Delay");
        assert_eq!(RuleKind::Redirect.legacy_processor_mode(), "Redirect");
        assert_eq!(
            RuleKind::RequestHeader.legacy_processor_mode(),
            "RequestHeader"
        );
        assert_eq!(RuleKind::Response.legacy_processor_mode(), "Response");
        assert_eq!(
            RuleKind::ResponseHeader.legacy_processor_mode(),
            "ResponseHeader"
        );
    }

    #[test]
    fn core_api_rules_validates_typed_rule_contract() {
        let request = SaveTypedRulesRequest {
            api_version: CORE_API_VERSION,
            pack_name: "default".to_string(),
            rules: vec![
                TypedRuleDefinition {
                    id: "request-rule".to_string(),
                    name: Some("route users".to_string()),
                    enabled: true,
                    priority: 20,
                    phase: RulePhase::Request,
                    matchers: vec![
                        RuleMatcher::Method {
                            value: "GET".to_string(),
                        },
                        RuleMatcher::Host {
                            matcher: TextMatcher::Regex {
                                pattern: "^api\\.example\\.test$".to_string(),
                            },
                        },
                        RuleMatcher::Path {
                            matcher: TextMatcher::Contains {
                                value: "/users".to_string(),
                            },
                        },
                    ],
                    actions: vec![
                        RuleAction::Redirect {
                            uri: "https://upstream.example.test/users".to_string(),
                        },
                        RuleAction::RequestHeader {
                            operation: HeaderMutation::Set,
                            name: "x-proxyman".to_string(),
                            value: Some("1".to_string()),
                        },
                    ],
                },
                TypedRuleDefinition {
                    id: "response-rule".to_string(),
                    name: None,
                    enabled: true,
                    priority: 10,
                    phase: RulePhase::Response,
                    matchers: vec![RuleMatcher::Status { value: 200 }],
                    actions: vec![RuleAction::ResponseHeader {
                        operation: HeaderMutation::Set,
                        name: "x-cache".to_string(),
                        value: Some("hit".to_string()),
                    }],
                },
            ],
        };

        validate_save_typed_rules_request(&request).expect("typed rules should be valid");
    }

    #[test]
    fn core_api_rules_rejects_invalid_typed_regex() {
        let err = validate_save_typed_rules_request(&SaveTypedRulesRequest {
            api_version: CORE_API_VERSION,
            pack_name: "default".to_string(),
            rules: vec![TypedRuleDefinition {
                id: "bad-regex".to_string(),
                name: None,
                enabled: true,
                priority: 0,
                phase: RulePhase::Request,
                matchers: vec![RuleMatcher::Host {
                    matcher: TextMatcher::Regex {
                        pattern: "[".to_string(),
                    },
                }],
                actions: vec![RuleAction::Delay { millis: 1 }],
            }],
        })
        .expect_err("invalid regex should fail validation");

        assert!(err.contains("Invalid rule matcher regex"));
    }

    #[test]
    fn core_api_rules_rejects_duplicate_typed_rule_ids() {
        let err = validate_save_typed_rules_request(&SaveTypedRulesRequest {
            api_version: CORE_API_VERSION,
            pack_name: "default".to_string(),
            rules: vec![
                typed_delay_rule("duplicate", 20),
                typed_delay_rule("duplicate", 10),
            ],
        })
        .expect_err("duplicate rule id should fail validation");

        assert_eq!(err, "Duplicate rule id: duplicate");
    }

    #[test]
    fn core_api_rules_rejects_response_action_in_request_phase() {
        let err = validate_save_typed_rules_request(&SaveTypedRulesRequest {
            api_version: CORE_API_VERSION,
            pack_name: "default".to_string(),
            rules: vec![TypedRuleDefinition {
                id: "wrong-phase".to_string(),
                name: None,
                enabled: true,
                priority: 0,
                phase: RulePhase::Request,
                matchers: vec![RuleMatcher::Path {
                    matcher: TextMatcher::Contains {
                        value: "/users".to_string(),
                    },
                }],
                actions: vec![RuleAction::ResponseHeader {
                    operation: HeaderMutation::Set,
                    name: "x-response".to_string(),
                    value: Some("no".to_string()),
                }],
            }],
        })
        .expect_err("response action should not be valid in request phase");

        assert_eq!(
            err,
            "Rule wrong-phase action responseHeader is not valid for request phase"
        );
    }

    #[test]
    fn core_api_rules_orders_typed_rules_deterministically() {
        let rules = vec![
            typed_delay_rule("b", 10),
            typed_delay_rule("c", 5),
            typed_delay_rule("a", 10),
        ];

        assert_eq!(
            typed_rule_evaluation_order(&rules),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn core_api_rules_parses_simple_line_dsl() {
        let rules = parse_typed_rules_dsl(
            r#"
            # action         match  url glob                    args
            redirect         GET    https://api.example.com/users*  https://mock.local/users
            delay            *      https://api.example.com/slow*   500ms
            request-header   GET    https://api.example.com/*       x-debug true
            response-header  200    https://api.example.com/*       x-cache hit
            block            *      https://ads.example.com/*       403
            "#,
        )
        .expect("rules dsl should parse");

        assert_eq!(rules.len(), 5);
        assert_eq!(rules[0].phase, RulePhase::Request);
        assert_eq!(rules[0].priority, 1_000_000);
        assert!(matches!(rules[0].actions[0], RuleAction::Redirect { .. }));
        assert!(matches!(
            rules[1].actions[0],
            RuleAction::Delay { millis: 500 }
        ));
        assert!(matches!(
            rules[2].actions[0],
            RuleAction::RequestHeader {
                operation: HeaderMutation::Set,
                ..
            }
        ));
        assert_eq!(rules[3].phase, RulePhase::Response);
        assert!(matches!(rules[3].matchers[0], RuleMatcher::Status { value: 200 }));
        assert!(matches!(
            rules[4].actions[0],
            RuleAction::Block { status: 403, .. }
        ));

        validate_save_typed_rules_request(&SaveTypedRulesRequest {
            api_version: CORE_API_VERSION,
            pack_name: "default".to_string(),
            rules,
        })
        .expect("parsed rules should validate");
    }

    #[test]
    fn core_api_rules_dsl_reports_line_errors() {
        let err = parse_typed_rules_dsl("delay * https://api.example.com/* nope")
            .expect_err("invalid duration should fail");

        assert_eq!(err, "Line 1: delay expects a duration like 500ms");
    }

    #[test]
    fn core_api_rules_formats_simple_line_dsl() {
        let rules = parse_typed_rules_dsl(
            r#"
            redirect GET https://api.example.com/users* https://mock.local/users
            delay * https://api.example.com/slow* 500ms
            request-header GET https://api.example.com/* x-debug true
            response-header 200 https://api.example.com/* x-cache hit
            block * https://ads.example.com/* 403
            "#,
        )
        .expect("rules dsl should parse");

        assert_eq!(
            format_typed_rules_dsl(&rules).expect("rules should format"),
            "redirect GET https://api.example.com/users* https://mock.local/users\n\
delay * https://api.example.com/slow* 500ms\n\
request-header GET https://api.example.com/* x-debug true\n\
response-header 200 https://api.example.com/* x-cache hit\n\
block * https://ads.example.com/* 403\n"
        );
    }

    #[test]
    fn core_api_rules_parses_and_formats_map_and_body_line_dsl() {
        let rules = parse_typed_rules_dsl(
            r#"
            map-remote   GET  https://api.example.com/*      https://upstream.example.com
            map-local    *    https://static.example.com/*   /tmp/static.json
            request-body POST https://api.example.com/users* patched request body
            response-body 201 https://api.example.com/users* patched response body
            "#,
        )
        .expect("map and body rules should parse");

        assert_eq!(rules.len(), 4);
        assert!(matches!(rules[0].actions[0], RuleAction::MapRemote { .. }));
        assert!(matches!(rules[1].actions[0], RuleAction::MapLocal { .. }));
        assert_eq!(
            rules[2].actions[0],
            RuleAction::RequestBody {
                body: "patched request body".to_string(),
            }
        );
        assert_eq!(rules[3].phase, RulePhase::Response);
        assert_eq!(
            rules[3].actions[0],
            RuleAction::ResponseBody {
                body: "patched response body".to_string(),
            }
        );

        assert_eq!(
            format_typed_rules_dsl(&rules).expect("rules should format"),
            "map-remote GET https://api.example.com/* https://upstream.example.com\n\
map-local * https://static.example.com/* /tmp/static.json\n\
request-body POST https://api.example.com/users* patched request body\n\
response-body 201 https://api.example.com/users* patched response body\n"
        );
    }

    #[test]
    fn core_api_rules_parses_url_glob_port_as_port_matcher() {
        let rules = parse_typed_rules_dsl(
            "response-header 200 http://127.0.0.1:8080/* x-cache hit",
        )
        .expect("rules dsl should parse");

        assert!(rules[0].matchers.iter().any(|matcher| {
            matches!(matcher, RuleMatcher::Host { matcher: TextMatcher::Regex { pattern } } if pattern == "^127\\.0\\.0\\.1$")
        }));
        assert!(rules[0]
            .matchers
            .iter()
            .any(|matcher| matches!(matcher, RuleMatcher::Port { value: 8080 })));
        assert_eq!(
            format_typed_rules_dsl(&rules).expect("rules should format"),
            "response-header 200 http://127.0.0.1:8080/* x-cache hit\n"
        );
    }

    #[test]
    fn core_api_rules_validates_rule_pack_text_request() {
        let request = SaveRulePackRulesRequest {
            api_version: CORE_API_VERSION,
            pack_name: "default".to_string(),
            enabled: true,
            content: "# keep this comment\nredirect GET https://api.example.com/* https://mock.local\n"
                .to_string(),
        };

        let rules =
            parse_save_rule_pack_rules_request(&request).expect("rules pack text should parse");

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].priority, 1_000_000);
        assert!(matches!(rules[0].actions[0], RuleAction::Redirect { .. }));
    }

    #[test]
    fn core_api_rules_rejects_invalid_rule_pack_text_request() {
        let err = parse_save_rule_pack_rules_request(&SaveRulePackRulesRequest {
            api_version: CORE_API_VERSION,
            pack_name: "default".to_string(),
            enabled: true,
            content: "delay * https://api.example.com/* nope".to_string(),
        })
        .expect_err("invalid rules pack text should fail");

        assert_eq!(err, "Line 1: delay expects a duration like 500ms");
    }

    #[test]
    fn core_api_rules_builds_rule_pack_text_response_without_rewriting_content() {
        let content =
            "# keep user spacing\nredirect   GET   https://api.example.com/*   https://mock.local\n"
                .to_string();

        let response = rule_pack_rules_response("default".to_string(), true, content.clone())
            .expect("rules pack response should parse");

        assert_eq!(response.content, content);
        assert_eq!(response.rules.len(), 1);
        assert_eq!(response.evaluation_order, vec![response.rules[0].id.clone()]);
    }

    fn typed_delay_rule(id: &str, priority: i32) -> TypedRuleDefinition {
        TypedRuleDefinition {
            id: id.to_string(),
            name: None,
            enabled: true,
            priority,
            phase: RulePhase::Request,
            matchers: vec![RuleMatcher::Path {
                matcher: TextMatcher::Contains {
                    value: "/".to_string(),
                },
            }],
            actions: vec![RuleAction::Delay { millis: 1 }],
        }
    }
}
