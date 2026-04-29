use chrono::{DateTime, TimeZone, Utc};
use http::{HeaderName, HeaderValue, Method, StatusCode, Uri};
use hyper::{body::to_bytes, Body, Client, Request};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

const SESSION_INLINE_BODY_LIMIT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Default)]
pub(crate) struct SessionSearchFilter {
    pub method: Option<String>,
    pub host: Option<String>,
    pub status: Option<u16>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplayRequestEdit {
    pub method: Option<String>,
    pub uri: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplayResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub(crate) struct SessionEventStore {
    file: File,
    body_dir: PathBuf,
}

impl SessionEventStore {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                log::error!("Create session event store dir failed, {err}");
                "Create session event store dir failed".to_string()
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| {
                log::error!("Open session event store failed, {err}");
                "Open session event store failed".to_string()
            })?;

        Ok(Self {
            file,
            body_dir: app_session_body_dir(path),
        })
    }

    pub(crate) fn append<T: Serialize>(&mut self, event: &T) -> Result<(), String> {
        let mut event = serde_json::to_value(event).map_err(|err| {
            log::error!("Serialize session event failed, {err}");
            "Serialize session event failed".to_string()
        })?;
        spill_large_bodies(&mut event, &self.body_dir)?;

        serde_json::to_writer(&mut self.file, &event).map_err(|err| {
            log::error!("Write serialized session event failed, {err}");
            "Write serialized session event failed".to_string()
        })?;
        self.file.write_all(b"\n").map_err(|err| {
            log::error!("Write session event delimiter failed, {err}");
            "Write session event delimiter failed".to_string()
        })?;
        self.file.flush().map_err(|err| {
            log::error!("Flush session event store failed, {err}");
            "Flush session event store failed".to_string()
        })
    }
}

fn spill_large_bodies(event: &mut Value, body_dir: &Path) -> Result<(), String> {
    match event {
        Value::Object(map) => {
            if let Some(Value::String(body)) = map.get("body") {
                if body.len() > SESSION_INLINE_BODY_LIMIT_BYTES {
                    fs::create_dir_all(body_dir).map_err(|err| {
                        log::error!("Create session body dir failed, {err}");
                        "Create session body dir failed".to_string()
                    })?;

                    let body_ref = format!("{}.body", uuid::Uuid::new_v4());
                    fs::write(body_dir.join(body_ref.as_str()), body.as_bytes()).map_err(
                        |err| {
                            log::error!("Write session sidecar body failed, {err}");
                            "Write session sidecar body failed".to_string()
                        },
                    )?;

                    let body_size = body.len();
                    let preview = truncate_utf8_at_byte_limit(body, SESSION_INLINE_BODY_LIMIT_BYTES);
                    map.insert("body".to_string(), Value::String(preview));
                    map.insert("bodyRef".to_string(), Value::String(body_ref));
                    map.insert("bodySize".to_string(), json!(body_size));
                    map.insert("bodyTruncated".to_string(), Value::Bool(true));
                }
            }

            for value in map.values_mut() {
                spill_large_bodies(value, body_dir)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                spill_large_bodies(value, body_dir)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn truncate_utf8_at_byte_limit(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_string();
    }

    let boundary = value
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= limit)
        .last()
        .unwrap_or(0);

    value[..boundary].to_string()
}

fn app_session_body_dir<P: AsRef<Path>>(event_log_path: P) -> PathBuf {
    event_log_path
        .as_ref()
        .parent()
        .map(|parent| parent.join("bodies"))
        .unwrap_or_else(|| PathBuf::from("bodies"))
}

pub(crate) fn load_session_body_ref_from_dir<P: AsRef<Path>>(
    body_dir: P,
    body_ref: &str,
) -> Result<String, String> {
    if body_ref.contains('/') || body_ref.contains('\\') || body_ref.contains("..") {
        return Err("Invalid body ref".to_string());
    }

    let body = fs::read_to_string(body_dir.as_ref().join(body_ref)).map_err(|err| {
        log::error!("Read session body ref failed, {err}");
        "Read session body ref failed".to_string()
    })?;

    Ok(body)
}

pub(crate) fn search_session_events_from_path<P: AsRef<Path>>(
    path: P,
    filter: &SessionSearchFilter,
) -> Result<Vec<Value>, String> {
    let file = File::open(path.as_ref()).map_err(|err| {
        log::error!("Open session event log failed, {err}");
        "Open session event log failed".to_string()
    })?;
    let reader = BufReader::new(file);
    let mut matches = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|err| {
            log::error!("Read session event line failed, {err}");
            "Read session event line failed".to_string()
        })?;
        if line.trim().is_empty() {
            continue;
        }

        let event = serde_json::from_str::<Value>(line.as_str()).map_err(|err| {
            log::error!("Parse session event line failed, {err}");
            "Parse session event line failed".to_string()
        })?;

        if event_matches(&event, filter) {
            matches.push(event);
        }
    }

    Ok(matches)
}

pub(crate) fn export_har_from_session_events_path<P: AsRef<Path>>(
    path: P,
) -> Result<Value, String> {
    let events = read_session_event_values(path)?;
    let mut exchange_order = Vec::<String>::new();
    let mut exchanges = HashMap::<String, ExchangeEvents>::new();

    for event in events {
        if let Some(request) = event.get("NewRequest") {
            if let Some(id) = event_id(request) {
                if !exchanges.contains_key(id) {
                    exchange_order.push(id.to_string());
                }
                exchanges
                    .entry(id.to_string())
                    .or_default()
                    .request
                    .replace(request.clone());
            }
            continue;
        }

        if let Some(response) = event.get("NewResponse") {
            if let Some(id) = event_id(response) {
                if !exchanges.contains_key(id) {
                    exchange_order.push(id.to_string());
                }
                exchanges
                    .entry(id.to_string())
                    .or_default()
                    .response
                    .replace(response.clone());
            }
        }
    }

    let entries = exchange_order
        .into_iter()
        .filter_map(|id| exchanges.remove(id.as_str()))
        .filter_map(|exchange| exchange.into_har_entry())
        .collect::<Vec<_>>();

    Ok(json!({
        "log": {
            "version": "1.2",
            "creator": {
                "name": "Proxyman",
                "version": env!("CARGO_PKG_VERSION")
            },
            "entries": entries
        }
    }))
}

pub(crate) fn import_har_to_session_events_path<P: AsRef<Path>>(
    path: P,
    har: Value,
) -> Result<usize, String> {
    let entries = har
        .get("log")
        .and_then(|log| log.get("entries"))
        .and_then(Value::as_array)
        .ok_or_else(|| "Invalid HAR: missing log.entries".to_string())?;
    let mut store = SessionEventStore::open(path)?;
    let mut imported = 0;

    for entry in entries {
        let Some(request) = entry.get("request") else {
            continue;
        };
        let exchange_id = uuid::Uuid::new_v4().to_string();
        let started_time = entry
            .get("startedDateTime")
            .and_then(Value::as_str)
            .and_then(parse_har_datetime_millis)
            .unwrap_or_else(|| chrono::Local::now().timestamp_millis() as u64);
        let duration = entry.get("time").and_then(Value::as_u64).unwrap_or(0);
        let response_time = started_time.saturating_add(duration);

        let request_event = json!({
            "NewRequest": {
                "id": exchange_id,
                "method": request.get("method").and_then(Value::as_str).unwrap_or("GET"),
                "uri": request.get("url").and_then(Value::as_str).unwrap_or_default(),
                "version": request.get("httpVersion").and_then(Value::as_str).unwrap_or("HTTP/1.1"),
                "headers": har_headers_to_event_headers(request.get("headers")),
                "body": request
                    .get("postData")
                    .and_then(|post_data| post_data.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                "time": started_time
            }
        });
        store.append(&request_event)?;

        if let Some(response) = entry.get("response") {
            let response_event = json!({
                "NewResponse": {
                    "id": exchange_id,
                    "uri": request.get("url").and_then(Value::as_str).unwrap_or_default(),
                    "status": response.get("status").and_then(Value::as_u64).unwrap_or(0),
                    "version": response.get("httpVersion").and_then(Value::as_str).unwrap_or("HTTP/1.1"),
                    "headers": har_headers_to_event_headers(response.get("headers")),
                    "body": response
                        .get("content")
                        .and_then(|content| content.get("text"))
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    "time": response_time
                }
            });
            store.append(&response_event)?;
        }

        imported += 1;
    }

    Ok(imported)
}

pub(crate) async fn build_replay_request_from_session_events_path<P: AsRef<Path>>(
    path: P,
    exchange_id: &str,
    edit: ReplayRequestEdit,
) -> Result<Request<Body>, String> {
    let request = find_request_event(read_session_event_values(path)?, exchange_id)
        .ok_or_else(|| format!("Request event not found for exchange {exchange_id}"))?;

    build_replay_request(request, edit)
}

pub(crate) async fn replay_session_request_from_path<P: AsRef<Path>>(
    path: P,
    exchange_id: &str,
    edit: ReplayRequestEdit,
) -> Result<ReplayResponse, String> {
    let request = build_replay_request_from_session_events_path(path, exchange_id, edit).await?;
    let client = Client::new();
    let response = client.request(request).await.map_err(|err| {
        log::error!("Replay request failed, {err}");
        format!("Replay request failed: {err}")
    })?;

    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect::<HashMap<_, _>>();
    let body = to_bytes(response.into_body()).await.map_err(|err| {
        log::error!("Read replay response body failed, {err}");
        "Read replay response body failed".to_string()
    })?;

    Ok(ReplayResponse {
        status,
        headers,
        body: String::from_utf8_lossy(body.as_ref()).into_owned(),
    })
}

fn build_replay_request(request: Value, edit: ReplayRequestEdit) -> Result<Request<Body>, String> {
    let method = edit.method.as_deref().or_else(|| request.get("method").and_then(Value::as_str));
    let method = method
        .ok_or_else(|| "Replay request missing method".to_string())?
        .parse::<Method>()
        .map_err(|err| format!("Invalid replay method: {err}"))?;

    let uri = edit.uri.as_deref().or_else(|| request.get("uri").and_then(Value::as_str));
    let uri = uri
        .ok_or_else(|| "Replay request missing uri".to_string())?
        .parse::<Uri>()
        .map_err(|err| format!("Invalid replay uri: {err}"))?;

    let body = edit
        .body
        .or_else(|| {
            request
                .get("body")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default();

    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(headers) = builder.headers_mut() {
        for (name, value) in replay_headers(request.get("headers"), edit.headers) {
            if should_skip_replay_header(name.as_str()) {
                continue;
            }

            let header_name = HeaderName::try_from(name.as_str())
                .map_err(|err| format!("Invalid replay header name({name}): {err}"))?;
            let header_value = HeaderValue::try_from(value.as_str())
                .map_err(|err| format!("Invalid replay header value({name}): {err}"))?;
            headers.insert(header_name, header_value);
        }
    }

    builder
        .body(Body::from(body))
        .map_err(|err| format!("Build replay request failed: {err}"))
}

fn read_session_event_values<P: AsRef<Path>>(path: P) -> Result<Vec<Value>, String> {
    let file = File::open(path.as_ref()).map_err(|err| {
        log::error!("Open session event log failed, {err}");
        "Open session event log failed".to_string()
    })?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|err| {
            log::error!("Read session event line failed, {err}");
            "Read session event line failed".to_string()
        })?;
        if line.trim().is_empty() {
            continue;
        }

        let event = serde_json::from_str::<Value>(line.as_str()).map_err(|err| {
            log::error!("Parse session event line failed, {err}");
            "Parse session event line failed".to_string()
        })?;
        events.push(event);
    }

    Ok(events)
}

fn find_request_event(events: Vec<Value>, exchange_id: &str) -> Option<Value> {
    events.into_iter().find_map(|event| {
        event.get("NewRequest").and_then(|request| {
            if event_id(request) == Some(exchange_id) {
                Some(request.clone())
            } else {
                None
            }
        })
    })
}

fn replay_headers(
    original: Option<&Value>,
    edits: Option<HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut headers = original
        .and_then(Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(name, value)| {
                    header_value_to_string(value).map(|value| (name.clone(), value))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    if let Some(edits) = edits {
        for (name, value) in edits {
            headers.insert(name, value);
        }
    }

    headers
}

fn header_value_to_string(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_string());
    }

    value.as_array().map(|values| {
        values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    })
}

fn should_skip_replay_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "host" | "content-length" | "transfer-encoding" | "connection"
    )
}

#[derive(Default)]
struct ExchangeEvents {
    request: Option<Value>,
    response: Option<Value>,
}

impl ExchangeEvents {
    fn into_har_entry(self) -> Option<Value> {
        let request = self.request?;
        let response = self.response.unwrap_or_else(|| json!({}));
        let request_time = event_time_millis(&request);
        let response_time = event_time_millis(&response);
        let total_time = request_time
            .zip(response_time)
            .map(|(request_time, response_time)| response_time.saturating_sub(request_time))
            .unwrap_or(0);
        let request_url = request
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let status = response
            .get("status")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;

        Some(json!({
            "startedDateTime": request_time
                .map(format_millis_as_rfc3339)
                .unwrap_or_default(),
            "time": total_time,
            "request": {
                "method": request.get("method").and_then(Value::as_str).unwrap_or_default(),
                "url": request_url,
                "httpVersion": request.get("version").and_then(Value::as_str).unwrap_or("HTTP/1.1"),
                "headers": headers_to_har(request.get("headers")),
                "queryString": query_string_to_har(request_url),
                "cookies": [],
                "headersSize": -1,
                "bodySize": request.get("body").and_then(Value::as_str).map(str::len).unwrap_or(0),
                "postData": {
                    "mimeType": "",
                    "text": request.get("body").and_then(Value::as_str).unwrap_or_default()
                }
            },
            "response": {
                "status": status,
                "statusText": status_text(status),
                "httpVersion": response.get("version").and_then(Value::as_str).unwrap_or("HTTP/1.1"),
                "headers": headers_to_har(response.get("headers")),
                "cookies": [],
                "content": {
                    "size": response.get("body").and_then(Value::as_str).map(str::len).unwrap_or(0),
                    "mimeType": response_content_type(&response),
                    "text": response.get("body").and_then(Value::as_str).unwrap_or_default()
                },
                "redirectURL": "",
                "headersSize": -1,
                "bodySize": response.get("body").and_then(Value::as_str).map(str::len).unwrap_or(0)
            },
            "cache": {},
            "timings": {
                "send": 0,
                "wait": total_time,
                "receive": 0
            }
        }))
    }
}

fn event_id(event: &Value) -> Option<&str> {
    event.get("id").and_then(Value::as_str)
}

fn headers_to_har(headers: Option<&Value>) -> Vec<Value> {
    headers
        .and_then(Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .map(|(name, value)| {
                    json!({
                        "name": name,
                        "value": value.as_str().unwrap_or_default()
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn response_content_type(response: &Value) -> &str {
    response
        .get("headers")
        .and_then(Value::as_object)
        .and_then(|headers| headers.get("content-type").or_else(|| headers.get("Content-Type")))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn har_headers_to_event_headers(headers: Option<&Value>) -> Value {
    let mut event_headers = serde_json::Map::new();

    if let Some(headers) = headers.and_then(Value::as_array) {
        for header in headers {
            let Some(name) = header.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(value) = header.get("value").and_then(Value::as_str) else {
                continue;
            };
            event_headers.insert(name.to_string(), Value::String(value.to_string()));
        }
    }

    Value::Object(event_headers)
}

fn parse_har_datetime_millis(value: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.timestamp_millis().max(0) as u64)
}

fn event_time_millis(event: &Value) -> Option<u64> {
    event.get("time").and_then(Value::as_u64)
}

fn format_millis_as_rfc3339(millis: u64) -> String {
    Utc.timestamp_millis_opt(millis as i64)
        .single()
        .map(|time| time.to_rfc3339())
        .unwrap_or_default()
}

fn query_string_to_har(uri: &str) -> Vec<Value> {
    uri.split_once('?')
        .map(|(_, query)| {
            query
                .split('&')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let (name, value) = part.split_once('=').unwrap_or((part, ""));
                    json!({
                        "name": name,
                        "value": value
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn status_text(status: u16) -> &'static str {
    StatusCode::from_u16(status)
        .ok()
        .and_then(|status| status.canonical_reason())
        .unwrap_or_default()
}

fn event_matches(event: &Value, filter: &SessionSearchFilter) -> bool {
    if let Some(request) = event.get("NewRequest") {
        return request_matches(request, filter);
    }

    if let Some(response) = event.get("NewResponse") {
        return response_matches(response, filter);
    }

    filter.method.is_none() && filter.status.is_none() && uri_matches(event.get("uri"), filter)
}

fn request_matches(request: &Value, filter: &SessionSearchFilter) -> bool {
    if let Some(method) = filter.method.as_deref() {
        if request.get("method").and_then(Value::as_str) != Some(method) {
            return false;
        }
    }

    if filter.status.is_some() {
        return false;
    }

    uri_matches(request.get("uri"), filter)
}

fn response_matches(response: &Value, filter: &SessionSearchFilter) -> bool {
    if filter.method.is_some() {
        return false;
    }

    if let Some(status) = filter.status {
        if response.get("status").and_then(Value::as_u64) != Some(status as u64) {
            return false;
        }
    }

    uri_matches(response.get("uri"), filter)
}

fn uri_matches(uri: Option<&Value>, filter: &SessionSearchFilter) -> bool {
    match filter.host.as_deref() {
        Some(host) => uri.and_then(Value::as_str).is_some_and(|uri| uri.contains(host)),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{
        body::to_bytes,
        service::{make_service_fn, service_fn},
        Body, Response, Server,
    };
    use serde_json::json;
    use std::net::SocketAddr;
    use tokio::sync::oneshot;

    #[test]
    fn session_store_writes_events_as_json_lines() {
        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");

        store
            .append(&json!({ "event": "request", "id": 1 }))
            .expect("failed to append first event");
        store
            .append(&json!({ "event": "response", "id": 1 }))
            .expect("failed to append second event");

        let content = std::fs::read_to_string(&path).expect("failed to read events");
        let lines = content.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(lines[0]).unwrap(),
            json!({ "event": "request", "id": 1 })
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(lines[1]).unwrap(),
            json!({ "event": "response", "id": 1 })
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn session_store_creates_parent_directories() {
        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("nested").join("events.jsonl");

        SessionEventStore::open(&path).expect("failed to open store");

        assert!(path.exists());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn session_store_spills_large_bodies_to_sidecar_file() {
        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");
        let body = "x".repeat(SESSION_INLINE_BODY_LIMIT_BYTES + 1);

        store
            .append(&json!({
                "NewResponse": {
                    "id": "exchange-1",
                    "uri": "http://api.example.test/users",
                    "status": 200,
                    "headers": {},
                    "body": body
                }
            }))
            .expect("failed to append response");

        let content = std::fs::read_to_string(&path).expect("failed to read events");
        let event = serde_json::from_str::<Value>(content.lines().next().unwrap())
            .expect("failed to parse event");
        let response = &event["NewResponse"];
        let body_ref = response["bodyRef"].as_str().expect("missing body ref");

        assert_eq!(
            response["body"].as_str().unwrap().len(),
            SESSION_INLINE_BODY_LIMIT_BYTES
        );
        assert_eq!(
            response["bodySize"],
            (SESSION_INLINE_BODY_LIMIT_BYTES + 1) as u64
        );
        assert_eq!(response["bodyTruncated"], true);
        assert_eq!(
            load_session_body_ref_from_dir(dir.join("bodies"), body_ref)
                .expect("failed to load body ref")
                .len(),
            SESSION_INLINE_BODY_LIMIT_BYTES + 1
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn session_store_search_filters_method_host_and_status() {
        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");

        store
            .append(&json!({
                "NewRequest": {
                    "method": "GET",
                    "uri": "http://api.example.test/users",
                    "headers": {},
                    "body": ""
                }
            }))
            .expect("failed to append request");
        store
            .append(&json!({
                "NewResponse": {
                    "uri": "http://api.example.test/users",
                    "status": 200,
                    "headers": {},
                    "body": ""
                }
            }))
            .expect("failed to append response");
        store
            .append(&json!({
                "NewRequest": {
                    "method": "POST",
                    "uri": "http://other.example.test/users",
                    "headers": {},
                    "body": ""
                }
            }))
            .expect("failed to append other request");

        let matches = search_session_events_from_path(
            &path,
            &SessionSearchFilter {
                method: Some("GET".to_string()),
                host: Some("api.example.test".to_string()),
                status: None,
            },
        )
        .expect("failed to search request events");
        assert_eq!(matches.len(), 1);

        let matches = search_session_events_from_path(
            &path,
            &SessionSearchFilter {
                method: None,
                host: Some("api.example.test".to_string()),
                status: Some(200),
            },
        )
        .expect("failed to search response events");
        assert_eq!(matches.len(), 1);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn session_store_exports_basic_har_entries() {
        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");

        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-1",
                    "method": "GET",
                    "uri": "http://api.example.test/users",
                    "headers": { "accept": "application/json" },
                    "body": ""
                }
            }))
            .expect("failed to append request");
        store
            .append(&json!({
                "NewResponse": {
                    "id": "exchange-1",
                    "uri": "http://api.example.test/users",
                    "status": 200,
                    "headers": { "content-type": "application/json" },
                    "body": "{\"ok\":true}"
                }
            }))
            .expect("failed to append response");

        let har = export_har_from_session_events_path(&path).expect("failed to export har");
        let entry = &har["log"]["entries"][0];

        assert_eq!(har["log"]["version"], "1.2");
        assert_eq!(entry["request"]["method"], "GET");
        assert_eq!(entry["request"]["url"], "http://api.example.test/users");
        assert_eq!(entry["response"]["status"], 200);
        assert_eq!(entry["response"]["content"]["text"], "{\"ok\":true}");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn session_store_exports_har_timings_and_query_string() {
        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");

        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-1",
                    "method": "GET",
                    "uri": "http://api.example.test/users?role=admin&active=true",
                    "headers": {},
                    "body": "",
                    "time": 1000
                }
            }))
            .expect("failed to append request");
        store
            .append(&json!({
                "NewResponse": {
                    "id": "exchange-1",
                    "uri": "http://api.example.test/users?role=admin&active=true",
                    "status": 201,
                    "headers": {},
                    "body": "created",
                    "time": 1450
                }
            }))
            .expect("failed to append response");

        let har = export_har_from_session_events_path(&path).expect("failed to export har");
        let entry = &har["log"]["entries"][0];

        assert_eq!(entry["startedDateTime"], "1970-01-01T00:00:01+00:00");
        assert_eq!(entry["time"], 450);
        assert_eq!(entry["timings"]["wait"], 450);
        assert_eq!(entry["response"]["statusText"], "Created");
        assert_eq!(
            entry["request"]["queryString"],
            json!([
                { "name": "role", "value": "admin" },
                { "name": "active", "value": "true" }
            ])
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn session_store_imports_har_entries_as_session_events() {
        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let imported = import_har_to_session_events_path(
            &path,
            json!({
                "log": {
                    "version": "1.2",
                    "entries": [
                        {
                            "startedDateTime": "1970-01-01T00:00:01+00:00",
                            "time": 250,
                            "request": {
                                "method": "POST",
                                "url": "http://api.example.test/users?role=admin",
                                "httpVersion": "HTTP/1.1",
                                "headers": [
                                    { "name": "content-type", "value": "application/json" }
                                ],
                                "postData": {
                                    "mimeType": "application/json",
                                    "text": "{\"name\":\"proxyman\"}"
                                }
                            },
                            "response": {
                                "status": 201,
                                "statusText": "Created",
                                "httpVersion": "HTTP/1.1",
                                "headers": [
                                    { "name": "content-type", "value": "application/json" }
                                ],
                                "content": {
                                    "mimeType": "application/json",
                                    "text": "{\"ok\":true}"
                                }
                            }
                        }
                    ]
                }
            }),
        )
        .expect("failed to import har");

        assert_eq!(imported, 1);

        let content = std::fs::read_to_string(&path).expect("failed to read events");
        assert_eq!(content.lines().count(), 2);

        let request_matches = search_session_events_from_path(
            &path,
            &SessionSearchFilter {
                method: Some("POST".to_string()),
                host: Some("api.example.test".to_string()),
                status: None,
            },
        )
        .expect("failed to search imported request");
        assert_eq!(request_matches.len(), 1);
        assert_eq!(request_matches[0]["NewRequest"]["time"], 1000);

        let response_matches = search_session_events_from_path(
            &path,
            &SessionSearchFilter {
                method: None,
                host: Some("api.example.test".to_string()),
                status: Some(201),
            },
        )
        .expect("failed to search imported response");
        assert_eq!(response_matches.len(), 1);
        assert_eq!(response_matches[0]["NewResponse"]["time"], 1250);

        let exchange_id = request_matches[0]["NewRequest"]["id"].as_str().unwrap();
        let replay_request = build_replay_request_from_session_events_path(
            &path,
            exchange_id,
            ReplayRequestEdit::default(),
        )
        .await
        .expect("failed to build replay request");
        assert_eq!(replay_request.method(), "POST");
        assert_eq!(
            to_bytes(replay_request.into_body()).await.unwrap(),
            "{\"name\":\"proxyman\"}"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn session_store_builds_replay_request_with_overrides() {
        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");

        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-1",
                    "method": "GET",
                    "uri": "http://api.example.test/users",
                    "headers": { "accept": "application/json" },
                    "body": ""
                }
            }))
            .expect("failed to append request");

        let request = build_replay_request_from_session_events_path(
            &path,
            "exchange-1",
            ReplayRequestEdit {
                method: Some("POST".to_string()),
                uri: Some("http://api.example.test/users/1".to_string()),
                headers: Some(HashMap::from([
                    ("content-type".to_string(), "application/json".to_string()),
                    ("x-replay".to_string(), "true".to_string()),
                ])),
                body: Some("{\"name\":\"proxyman\"}".to_string()),
            },
        )
        .await
        .expect("failed to build replay request");

        assert_eq!(request.method(), "POST");
        assert_eq!(request.uri(), "http://api.example.test/users/1");
        assert_eq!(request.headers().get("x-replay").unwrap(), "true");
        assert_eq!(
            to_bytes(request.into_body()).await.unwrap(),
            "{\"name\":\"proxyman\"}"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn session_store_replay_sends_edited_request_to_origin() {
        let origin_addr = reserve_local_addr();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let make_service = make_service_fn(|_| async {
            Ok::<_, hyper::Error>(service_fn(|req: hyper::Request<Body>| async move {
                let method = req.method().to_string();
                let path = req.uri().path().to_string();
                let header = req
                    .headers()
                    .get("x-replay")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("missing")
                    .to_string();
                let body = to_bytes(req.into_body()).await.unwrap_or_default();
                let body = String::from_utf8_lossy(body.as_ref()).into_owned();

                Ok::<_, hyper::Error>(Response::new(Body::from(format!(
                    "{method} {path} {header} {body}"
                ))))
            }))
        });
        let origin = Server::bind(&origin_addr)
            .serve(make_service)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            });
        tokio::spawn(origin);

        let dir = std::env::temp_dir().join(format!("proxyman-session-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");
        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-1",
                    "method": "GET",
                    "uri": format!("http://{origin_addr}/old"),
                    "headers": {},
                    "body": ""
                }
            }))
            .expect("failed to append request");

        let response = replay_session_request_from_path(
            &path,
            "exchange-1",
            ReplayRequestEdit {
                method: Some("POST".to_string()),
                uri: Some(format!("http://{origin_addr}/new")),
                headers: Some(HashMap::from([(
                    "x-replay".to_string(),
                    "matched".to_string(),
                )])),
                body: Some("body-ok".to_string()),
            },
        )
        .await
        .expect("failed to replay request");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "POST /new matched body-ok");

        let _ = shutdown_tx.send(());
        let _ = std::fs::remove_dir_all(dir);
    }

    fn reserve_local_addr() -> SocketAddr {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("failed to reserve local port");
        let addr = listener.local_addr().expect("failed to read local addr");
        drop(listener);
        addr
    }
}
