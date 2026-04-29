use async_trait::async_trait;
use http::{HeaderName, HeaderValue, Method, Uri};
use hyper::{body::to_bytes, Body, Client, Request};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::core_api::{
    events::{
        BodyChunkEvent, CapturedBodySummary, CoreEvent, CoreEventEnvelope, HeaderEntry,
        RequestHeadEvent, ResponseHeadEvent,
    },
    ensure_supported_version, CORE_API_VERSION,
};
use crate::storage::{
    export_har_from_session_events_path, import_har_to_session_events_path,
    load_session_body_ref_from_dir, replay_session_request_from_path, ReplayRequestEdit,
    SessionSearchFilter,
};

#[derive(Debug, Clone)]
pub(crate) struct CoreSessionApi<S> {
    store: S,
}

impl<S> CoreSessionApi<S> {
    pub(crate) fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S> CoreSessionApi<S>
where
    S: SessionStore,
{
    pub(crate) fn search_exchanges(
        &self,
        request: SearchSessionExchangesRequest,
    ) -> Result<SearchSessionExchangesResponse, String> {
        ensure_supported_version(request.api_version)?;

        let exchanges = self.store.search_exchanges(&request.filter)?;

        Ok(SearchSessionExchangesResponse {
            api_version: CORE_API_VERSION,
            exchanges,
        })
    }

    pub(crate) fn load_body(
        &self,
        request: LoadSessionBodyRequest,
    ) -> Result<LoadSessionBodyResponse, String> {
        ensure_supported_version(request.api_version)?;

        Ok(LoadSessionBodyResponse {
            api_version: CORE_API_VERSION,
            body: self.store.load_body(request.body_ref.as_str())?,
        })
    }

    pub(crate) fn export_har(
        &self,
        request: ExportSessionHarRequest,
    ) -> Result<ExportSessionHarResponse, String> {
        ensure_supported_version(request.api_version)?;

        Ok(ExportSessionHarResponse {
            api_version: CORE_API_VERSION,
            har: self.store.export_har()?,
        })
    }

    pub(crate) fn import_har(
        &self,
        request: ImportSessionHarRequest,
    ) -> Result<ImportSessionHarResponse, String> {
        ensure_supported_version(request.api_version)?;

        Ok(ImportSessionHarResponse {
            api_version: CORE_API_VERSION,
            imported: self.store.import_har(request.har)?,
        })
    }

    pub(crate) fn clear(
        &self,
        request: ClearSessionRequest,
    ) -> Result<ClearSessionResponse, String> {
        ensure_supported_version(request.api_version)?;

        Ok(ClearSessionResponse {
            api_version: CORE_API_VERSION,
            cleared: self.store.clear()?,
        })
    }

    pub(crate) async fn replay(
        &self,
        request: ReplaySessionRequest,
    ) -> Result<ReplaySessionResponse, String> {
        ensure_supported_version(request.api_version)?;

        let result = self
            .store
            .replay_request(request.exchange_id.as_str(), request.edit)
            .await?;

        Ok(ReplaySessionResponse {
            api_version: CORE_API_VERSION,
            status: result.status,
            headers: result.headers,
            body: result.body,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionReplayResult {
    pub(crate) status: u16,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: String,
}

#[async_trait]
pub(crate) trait SessionStore {
    fn search_exchanges(
        &self,
        filter: &SessionExchangeFilter,
    ) -> Result<Vec<SessionExchangeSummary>, String>;
    fn load_body(&self, body_ref: &str) -> Result<String, String>;
    fn clear(&self) -> Result<usize, String>;
    fn export_har(&self) -> Result<Value, String>;
    fn import_har(&self, har: Value) -> Result<usize, String>;
    async fn replay_request(
        &self,
        exchange_id: &str,
        edit: ReplayEdit,
    ) -> Result<SessionReplayResult, String>;
}

#[derive(Debug, Clone)]
pub(crate) struct JsonlSessionStore {
    event_log_path: PathBuf,
    body_dir: PathBuf,
}

impl JsonlSessionStore {
    pub(crate) fn new<P: AsRef<Path>>(event_log_path: P) -> Self {
        let event_log_path = event_log_path.as_ref().to_path_buf();
        let body_dir = event_log_path
            .parent()
            .map(|parent| parent.join("bodies"))
            .unwrap_or_else(|| PathBuf::from("bodies"));

        Self {
            event_log_path,
            body_dir,
        }
    }
}

#[async_trait]
impl SessionStore for JsonlSessionStore {
    fn search_exchanges(
        &self,
        filter: &SessionExchangeFilter,
    ) -> Result<Vec<SessionExchangeSummary>, String> {
        let events = crate::storage::search_session_events_from_path(
            &self.event_log_path,
            &SessionSearchFilter::default(),
        )?;
        let exchanges = summarize_exchange_records(events)
            .into_iter()
            .filter(|exchange| exchange_matches(exchange, filter))
            .map(|exchange| exchange.summary)
            .collect::<Vec<_>>();

        Ok(exchanges)
    }

    fn load_body(&self, body_ref: &str) -> Result<String, String> {
        load_session_body_ref_from_dir(&self.body_dir, body_ref)
    }

    fn clear(&self) -> Result<usize, String> {
        let cleared = crate::storage::search_session_events_from_path(
            &self.event_log_path,
            &SessionSearchFilter::default(),
        )
        .map(|events| summarize_exchange_records(events).len())
        .unwrap_or(0);

        if let Some(parent) = self.event_log_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                log::error!("Create session event log dir failed, {err}");
                "Create session event log dir failed".to_string()
            })?;
        }
        fs::write(&self.event_log_path, "").map_err(|err| {
            log::error!("Clear session event log failed, {err}");
            "Clear session event log failed".to_string()
        })?;

        match fs::remove_dir_all(&self.body_dir) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                log::error!("Clear session body dir failed, {err}");
                return Err("Clear session body dir failed".to_string());
            }
        }

        Ok(cleared)
    }

    fn export_har(&self) -> Result<Value, String> {
        export_har_from_session_events_path(&self.event_log_path)
    }

    fn import_har(&self, har: Value) -> Result<usize, String> {
        import_har_to_session_events_path(&self.event_log_path, har)
    }

    async fn replay_request(
        &self,
        exchange_id: &str,
        edit: ReplayEdit,
    ) -> Result<SessionReplayResult, String> {
        let response =
            replay_session_request_from_path(&self.event_log_path, exchange_id, edit.into())
                .await?;

        Ok(SessionReplayResult {
            status: response.status,
            headers: response.headers,
            body: response.body,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct InMemorySessionStore {
    state: Arc<Mutex<InMemorySessionState>>,
}

#[derive(Debug, Default)]
struct InMemorySessionState {
    order: Vec<String>,
    exchanges: HashMap<String, InMemoryExchange>,
}

#[derive(Debug, Clone, Default)]
struct InMemoryExchange {
    summary: SessionExchangeSummary,
    request_headers: Vec<(String, String)>,
    response_headers: Vec<(String, String)>,
    request_body: String,
    response_body: String,
    request_body_source: BodySource,
    response_body_source: BodySource,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum BodySource {
    #[default]
    Empty,
    Summary,
    Chunks,
}

impl InMemorySessionStore {
    pub(crate) fn apply_event(&self, envelope: CoreEventEnvelope) -> Result<(), String> {
        ensure_supported_version(envelope.api_version)?;

        let mut state = self
            .state
            .lock()
            .map_err(|_| "In-memory session store lock poisoned".to_string())?;

        match envelope.event {
            CoreEvent::ExchangeStarted(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                exchange.summary.method = Some(event.method);
                exchange.summary.uri = Some(event.uri.clone());
                exchange.summary.host = uri_host(event.uri.as_str());
                exchange.summary.request_time = Some(event.timestamp);
            }
            CoreEvent::RequestHead(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                apply_request_head(exchange, event);
            }
            CoreEvent::RequestBodyChunk(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                apply_body_chunk(
                    &mut exchange.request_body,
                    &mut exchange.request_body_source,
                    &event,
                );
                refresh_body_refs(exchange);
            }
            CoreEvent::RequestFinished(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                if let Some(body) = event.captured_body {
                    apply_body_summary(
                        &mut exchange.request_body,
                        &mut exchange.request_body_source,
                        body,
                    );
                    refresh_body_refs(exchange);
                }
            }
            CoreEvent::ResponseHead(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                apply_response_head(exchange, event);
            }
            CoreEvent::ResponseBodyChunk(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                apply_body_chunk(
                    &mut exchange.response_body,
                    &mut exchange.response_body_source,
                    &event,
                );
                refresh_body_refs(exchange);
            }
            CoreEvent::ResponseFinished(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                exchange.summary.status = Some(event.status);
                exchange.summary.response_time = Some(event.timestamp);
                if let Some(body) = event.captured_body {
                    apply_body_summary(
                        &mut exchange.response_body,
                        &mut exchange.response_body_source,
                        body,
                    );
                    refresh_body_refs(exchange);
                }
            }
            CoreEvent::ExchangeFinished(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                exchange.summary.status = event.status.or(exchange.summary.status);
                exchange.summary.response_time = Some(event.timestamp);
            }
            CoreEvent::ExchangeError(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                if exchange.summary.uri.is_none() {
                    exchange.summary.uri = event.uri.clone();
                    exchange.summary.host = event.uri.as_deref().and_then(uri_host);
                }
                exchange.summary.response_time = Some(event.timestamp);
            }
            CoreEvent::SseEvent(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                if exchange.summary.uri.is_none() {
                    exchange.summary.uri = Some(event.uri.clone());
                    exchange.summary.host = uri_host(event.uri.as_str());
                }
                append_text_body(
                    &mut exchange.response_body,
                    &mut exchange.response_body_source,
                    format!("event: {}\ndata: {}", event.event.unwrap_or_default(), event.data),
                );
                refresh_body_refs(exchange);
            }
            CoreEvent::WebSocketMessage(event) => {
                let exchange = state.exchange_mut(event.exchange_id.as_str());
                if exchange.summary.uri.is_none() {
                    exchange.summary.uri = Some(event.uri.clone());
                    exchange.summary.host = uri_host(event.uri.as_str());
                }
                append_text_body(
                    &mut exchange.response_body,
                    &mut exchange.response_body_source,
                    format!(
                        "{} {}: {}",
                        event.direction, event.opcode, event.payload_preview
                    ),
                );
                refresh_body_refs(exchange);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    fn search_exchanges(
        &self,
        filter: &SessionExchangeFilter,
    ) -> Result<Vec<SessionExchangeSummary>, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "In-memory session store lock poisoned".to_string())?;
        Ok(state
            .order
            .iter()
            .filter_map(|exchange_id| state.exchanges.get(exchange_id))
            .filter(|exchange| exchange_matches(&exchange.as_record(), filter))
            .map(|exchange| exchange.summary.clone())
            .collect())
    }

    fn load_body(&self, body_ref: &str) -> Result<String, String> {
        let (exchange_id, phase) = parse_memory_body_ref(body_ref)?;
        let state = self
            .state
            .lock()
            .map_err(|_| "In-memory session store lock poisoned".to_string())?;
        let exchange = state
            .exchanges
            .get(exchange_id)
            .ok_or_else(|| "Session exchange body not found".to_string())?;

        match phase {
            "request" => Ok(exchange.request_body.clone()),
            "response" => Ok(exchange.response_body.clone()),
            _ => Err("Invalid memory body ref".to_string()),
        }
    }

    fn clear(&self) -> Result<usize, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "In-memory session store lock poisoned".to_string())?;
        let cleared = state.exchanges.len();
        state.order.clear();
        state.exchanges.clear();
        Ok(cleared)
    }

    fn export_har(&self) -> Result<Value, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "In-memory session store lock poisoned".to_string())?;
        let entries = state
            .order
            .iter()
            .filter_map(|exchange_id| state.exchanges.get(exchange_id))
            .filter_map(InMemoryExchange::to_har_entry)
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

    fn import_har(&self, har: Value) -> Result<usize, String> {
        let entries = har
            .get("log")
            .and_then(|log| log.get("entries"))
            .and_then(Value::as_array)
            .ok_or_else(|| "Invalid HAR: missing log.entries".to_string())?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "In-memory session store lock poisoned".to_string())?;
        let mut imported = 0;

        for entry in entries {
            let Some(request) = entry.get("request") else {
                continue;
            };
            let exchange_id = uuid::Uuid::new_v4().to_string();
            let uri = request
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let request_time = entry
                .get("startedDateTime")
                .and_then(Value::as_str)
                .and_then(parse_rfc3339_millis_i64);
            let response_time = request_time.zip(entry.get("time").and_then(Value::as_i64)).map(
                |(request_time, duration)| request_time.saturating_add(duration),
            );
            let mut exchange = InMemoryExchange {
                summary: SessionExchangeSummary {
                    exchange_id: exchange_id.clone(),
                    method: request
                        .get("method")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    uri: Some(uri.clone()),
                    host: uri_host(uri.as_str()),
                    status: entry
                        .get("response")
                        .and_then(|response| response.get("status"))
                        .and_then(Value::as_u64)
                        .and_then(|status| u16::try_from(status).ok()),
                    request_time,
                    response_time,
                    request_body_ref: None,
                    response_body_ref: None,
                },
                request_headers: har_header_pairs(request.get("headers")),
                response_headers: entry
                    .get("response")
                    .map(|response| har_header_pairs(response.get("headers")))
                    .unwrap_or_default(),
                request_body: request
                    .get("postData")
                    .and_then(|post_data| post_data.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                response_body: entry
                    .get("response")
                    .and_then(|response| response.get("content"))
                    .and_then(|content| content.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                request_body_source: BodySource::Summary,
                response_body_source: BodySource::Summary,
            };
            refresh_body_refs(&mut exchange);
            state.order.push(exchange_id.clone());
            state.exchanges.insert(exchange_id, exchange);
            imported += 1;
        }

        Ok(imported)
    }

    async fn replay_request(
        &self,
        exchange_id: &str,
        edit: ReplayEdit,
    ) -> Result<SessionReplayResult, String> {
        let exchange = {
            let state = self
                .state
                .lock()
                .map_err(|_| "In-memory session store lock poisoned".to_string())?;
            state
                .exchanges
                .get(exchange_id)
                .cloned()
                .ok_or_else(|| format!("Session exchange not found for replay: {exchange_id}"))?
        };
        let request = build_in_memory_replay_request(&exchange, edit)?;
        send_replay_request(request).await
    }
}

impl InMemorySessionState {
    fn exchange_mut(&mut self, exchange_id: &str) -> &mut InMemoryExchange {
        if !self.exchanges.contains_key(exchange_id) {
            self.order.push(exchange_id.to_string());
            self.exchanges.insert(
                exchange_id.to_string(),
                InMemoryExchange {
                    summary: empty_summary(exchange_id),
                    ..InMemoryExchange::default()
                },
            );
        }

        self.exchanges
            .get_mut(exchange_id)
            .expect("exchange should exist after insertion")
    }
}

impl InMemoryExchange {
    fn as_record(&self) -> SessionExchangeRecord {
        SessionExchangeRecord {
            summary: self.summary.clone(),
            request_body: (!self.request_body.is_empty()).then(|| self.request_body.clone()),
            response_body: (!self.response_body.is_empty()).then(|| self.response_body.clone()),
            headers: self
                .request_headers
                .iter()
                .chain(self.response_headers.iter())
                .cloned()
                .collect(),
        }
    }

    fn to_har_entry(&self) -> Option<Value> {
        let uri = self.summary.uri.as_deref()?;
        let total_time = self
            .summary
            .request_time
            .zip(self.summary.response_time)
            .map(|(request_time, response_time)| response_time.saturating_sub(request_time))
            .unwrap_or(0);

        Some(json!({
            "startedDateTime": self.summary.request_time.map(format_millis_as_rfc3339_string).unwrap_or_default(),
            "time": total_time,
            "request": {
                "method": self.summary.method.as_deref().unwrap_or("GET"),
                "url": uri,
                "httpVersion": "HTTP/1.1",
                "headers": header_pairs_to_har(&self.request_headers),
                "queryString": query_string_pairs_to_har(uri),
                "cookies": [],
                "headersSize": -1,
                "bodySize": self.request_body.len(),
                "postData": {
                    "mimeType": "",
                    "text": self.request_body
                }
            },
            "response": {
                "status": self.summary.status.unwrap_or(0),
                "statusText": "",
                "httpVersion": "HTTP/1.1",
                "headers": header_pairs_to_har(&self.response_headers),
                "cookies": [],
                "content": {
                    "size": self.response_body.len(),
                    "mimeType": "",
                    "text": self.response_body
                },
                "redirectURL": "",
                "headersSize": -1,
                "bodySize": self.response_body.len()
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

fn apply_request_head(exchange: &mut InMemoryExchange, event: RequestHeadEvent) {
    exchange.summary.method = Some(event.method);
    exchange.summary.uri = Some(event.uri.clone());
    exchange.summary.host = uri_host(event.uri.as_str());
    exchange.summary.request_time = Some(event.timestamp);
    exchange.request_headers = header_entries_to_pairs(event.headers);
    if let Some(body) = event.captured_body {
        apply_body_summary(
            &mut exchange.request_body,
            &mut exchange.request_body_source,
            body,
        );
    }
    refresh_body_refs(exchange);
}

fn apply_response_head(exchange: &mut InMemoryExchange, event: ResponseHeadEvent) {
    if exchange.summary.uri.is_none() {
        exchange.summary.uri = Some(event.uri.clone());
        exchange.summary.host = uri_host(event.uri.as_str());
    }
    exchange.summary.status = Some(event.status);
    exchange.summary.response_time = Some(event.timestamp);
    exchange.response_headers = header_entries_to_pairs(event.headers);
    if let Some(body) = event.captured_body {
        apply_body_summary(
            &mut exchange.response_body,
            &mut exchange.response_body_source,
            body,
        );
    }
    refresh_body_refs(exchange);
}

fn apply_body_summary(body: &mut String, source: &mut BodySource, summary: CapturedBodySummary) {
    if *source == BodySource::Empty {
        *body = summary.preview;
        *source = BodySource::Summary;
    }
}

fn apply_body_chunk(body: &mut String, source: &mut BodySource, event: &BodyChunkEvent) {
    if *source == BodySource::Summary && event.offset == 0 {
        body.clear();
    }
    if *source != BodySource::Chunks {
        *source = BodySource::Chunks;
    }

    if event.offset == 0 && body.is_empty() {
        body.push_str(event.preview.as_str());
    } else {
        body.push_str(event.preview.as_str());
    }
}

fn append_text_body(body: &mut String, source: &mut BodySource, value: String) {
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(value.as_str());
    if *source == BodySource::Empty {
        *source = BodySource::Chunks;
    }
}

fn refresh_body_refs(exchange: &mut InMemoryExchange) {
    exchange.summary.request_body_ref = (!exchange.request_body.is_empty())
        .then(|| memory_body_ref(exchange.summary.exchange_id.as_str(), "request"));
    exchange.summary.response_body_ref = (!exchange.response_body.is_empty())
        .then(|| memory_body_ref(exchange.summary.exchange_id.as_str(), "response"));
}

fn memory_body_ref(exchange_id: &str, phase: &str) -> String {
    format!("memory://{exchange_id}/{phase}")
}

fn parse_memory_body_ref(body_ref: &str) -> Result<(&str, &str), String> {
    let value = body_ref
        .strip_prefix("memory://")
        .ok_or_else(|| "Invalid memory body ref".to_string())?;
    let (exchange_id, phase) = value
        .rsplit_once('/')
        .ok_or_else(|| "Invalid memory body ref".to_string())?;
    if exchange_id.is_empty() || phase.is_empty() || exchange_id.contains("..") {
        return Err("Invalid memory body ref".to_string());
    }

    Ok((exchange_id, phase))
}

fn build_in_memory_replay_request(
    exchange: &InMemoryExchange,
    edit: ReplayEdit,
) -> Result<Request<Body>, String> {
    let method = edit
        .method
        .as_deref()
        .or(exchange.summary.method.as_deref())
        .ok_or_else(|| "Replay request missing method".to_string())?
        .parse::<Method>()
        .map_err(|err| format!("Invalid replay method: {err}"))?;

    let uri = edit
        .uri
        .as_deref()
        .or(exchange.summary.uri.as_deref())
        .ok_or_else(|| "Replay request missing uri".to_string())?
        .parse::<Uri>()
        .map_err(|err| format!("Invalid replay uri: {err}"))?;

    let body = edit.body.unwrap_or_else(|| exchange.request_body.clone());
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(headers) = builder.headers_mut() {
        for (name, value) in replay_headers_from_pairs(&exchange.request_headers, edit.headers) {
            if should_skip_replay_header_name(name.as_str()) {
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

async fn send_replay_request(request: Request<Body>) -> Result<SessionReplayResult, String> {
    let client = Client::new();
    let response = client
        .request(request)
        .await
        .map_err(|err| format!("Replay request failed: {err}"))?;

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
    let body = to_bytes(response.into_body())
        .await
        .map_err(|err| format!("Read replay response body failed: {err}"))?;

    Ok(SessionReplayResult {
        status,
        headers,
        body: String::from_utf8_lossy(body.as_ref()).into_owned(),
    })
}

fn replay_headers_from_pairs(
    original: &[(String, String)],
    edits: Option<HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut headers = original.iter().cloned().collect::<HashMap<_, _>>();
    if let Some(edits) = edits {
        for (name, value) in edits {
            headers.insert(name, value);
        }
    }
    headers
}

fn should_skip_replay_header_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "host" | "content-length" | "transfer-encoding" | "connection"
    )
}

fn header_entries_to_pairs(headers: Vec<HeaderEntry>) -> Vec<(String, String)> {
    headers
        .into_iter()
        .map(|header| (header.name, header.value))
        .collect()
}

fn header_pairs_to_har(headers: &[(String, String)]) -> Vec<Value> {
    headers
        .iter()
        .map(|(name, value)| json!({ "name": name, "value": value }))
        .collect()
}

fn har_header_pairs(headers: Option<&Value>) -> Vec<(String, String)> {
    headers
        .and_then(Value::as_array)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|header| {
                    let name = header.get("name").and_then(Value::as_str)?;
                    let value = header.get("value").and_then(Value::as_str)?;
                    Some((name.to_string(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_rfc3339_millis_i64(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.timestamp_millis())
}

fn format_millis_as_rfc3339_string(millis: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis)
        .map(|time| time.to_rfc3339())
        .unwrap_or_default()
}

fn query_string_pairs_to_har(uri: &str) -> Vec<Value> {
    uri.split_once('?')
        .map(|(_, query)| {
            query
                .split('&')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let (name, value) = part.split_once('=').unwrap_or((part, ""));
                    json!({ "name": name, "value": value })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchSessionExchangesRequest {
    pub(crate) api_version: u16,
    pub(crate) filter: SessionExchangeFilter,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionExchangeFilter {
    pub(crate) method: Option<String>,
    pub(crate) host: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) status: Option<u16>,
    pub(crate) body_text: Option<String>,
    pub(crate) header_name: Option<String>,
    pub(crate) header_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchSessionExchangesResponse {
    pub(crate) api_version: u16,
    pub(crate) exchanges: Vec<SessionExchangeSummary>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionExchangeSummary {
    pub(crate) exchange_id: String,
    pub(crate) method: Option<String>,
    pub(crate) uri: Option<String>,
    pub(crate) host: Option<String>,
    pub(crate) status: Option<u16>,
    pub(crate) request_time: Option<i64>,
    pub(crate) response_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) request_body_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) response_body_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadSessionBodyRequest {
    pub(crate) api_version: u16,
    pub(crate) body_ref: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadSessionBodyResponse {
    pub(crate) api_version: u16,
    pub(crate) body: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportSessionHarRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportSessionHarResponse {
    pub(crate) api_version: u16,
    pub(crate) har: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportSessionHarRequest {
    pub(crate) api_version: u16,
    pub(crate) har: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportSessionHarResponse {
    pub(crate) api_version: u16,
    pub(crate) imported: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClearSessionRequest {
    pub(crate) api_version: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClearSessionResponse {
    pub(crate) api_version: u16,
    pub(crate) cleared: usize,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplayEdit {
    pub(crate) method: Option<String>,
    pub(crate) uri: Option<String>,
    pub(crate) headers: Option<HashMap<String, String>>,
    pub(crate) body: Option<String>,
}

impl From<ReplayEdit> for ReplayRequestEdit {
    fn from(value: ReplayEdit) -> Self {
        Self {
            method: value.method,
            uri: value.uri,
            headers: value.headers,
            body: value.body,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplaySessionRequest {
    pub(crate) api_version: u16,
    pub(crate) exchange_id: String,
    pub(crate) edit: ReplayEdit,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplaySessionResponse {
    pub(crate) api_version: u16,
    pub(crate) status: u16,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionExchangeRecord {
    summary: SessionExchangeSummary,
    request_body: Option<String>,
    response_body: Option<String>,
    headers: Vec<(String, String)>,
}

fn summarize_exchange_records(events: Vec<Value>) -> Vec<SessionExchangeRecord> {
    let mut summaries = HashMap::<String, SessionExchangeRecord>::new();
    let mut order = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();

    for event in events {
        if let Some(request) = event.get("NewRequest") {
            if let Some(exchange_id) = event_exchange_id(request) {
                push_exchange_order(exchange_id, &mut order, &mut seen);
                let record = summaries
                    .entry(exchange_id.to_string())
                    .or_insert_with(|| empty_record(exchange_id));
                record.summary.method = request
                    .get("method")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                record.summary.uri = request
                    .get("uri")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                record.summary.host = record.summary.uri.as_deref().and_then(uri_host);
                record.summary.request_time = event_time(request);
                record.request_body = request
                    .get("body")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                record.summary.request_body_ref = request
                    .get("bodyRef")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                record.headers.extend(header_pairs(request.get("headers")));
            }
            continue;
        }

        if let Some(response) = event.get("NewResponse") {
            if let Some(exchange_id) = event_exchange_id(response) {
                push_exchange_order(exchange_id, &mut order, &mut seen);
                let record = summaries
                    .entry(exchange_id.to_string())
                    .or_insert_with(|| empty_record(exchange_id));
                if record.summary.uri.is_none() {
                    record.summary.uri = response
                        .get("uri")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);
                    record.summary.host = record.summary.uri.as_deref().and_then(uri_host);
                }
                record.summary.status = response
                    .get("status")
                    .and_then(Value::as_u64)
                    .and_then(|status| u16::try_from(status).ok());
                record.summary.response_time = event_time(response);
                record.response_body = response
                    .get("body")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                record.summary.response_body_ref = response
                    .get("bodyRef")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                record.headers.extend(header_pairs(response.get("headers")));
            }
        }
    }

    order
        .into_iter()
        .filter_map(|exchange_id| summaries.remove(exchange_id.as_str()))
        .collect()
}

fn push_exchange_order(exchange_id: &str, order: &mut Vec<String>, seen: &mut HashSet<String>) {
    if seen.insert(exchange_id.to_string()) {
        order.push(exchange_id.to_string());
    }
}

fn empty_record(exchange_id: &str) -> SessionExchangeRecord {
    SessionExchangeRecord {
        summary: empty_summary(exchange_id),
        request_body: None,
        response_body: None,
        headers: Vec::new(),
    }
}

fn empty_summary(exchange_id: &str) -> SessionExchangeSummary {
    SessionExchangeSummary {
        exchange_id: exchange_id.to_string(),
        method: None,
        uri: None,
        host: None,
        status: None,
        request_time: None,
        response_time: None,
        request_body_ref: None,
        response_body_ref: None,
    }
}

fn event_exchange_id(event: &Value) -> Option<&str> {
    event.get("id").and_then(Value::as_str)
}

fn event_time(event: &Value) -> Option<i64> {
    event.get("time").and_then(Value::as_i64)
}

fn uri_host(uri: &str) -> Option<String> {
    uri.parse::<http::Uri>()
        .ok()
        .and_then(|uri| uri.host().map(ToOwned::to_owned))
}

fn exchange_matches(exchange: &SessionExchangeRecord, filter: &SessionExchangeFilter) -> bool {
    if let Some(method) = filter.method.as_deref() {
        if exchange.summary.method.as_deref() != Some(method) {
            return false;
        }
    }

    if let Some(host) = filter.host.as_deref() {
        if exchange.summary.host.as_deref() != Some(host) {
            return false;
        }
    }

    if let Some(path) = filter.path.as_deref() {
        if !exchange
            .summary
            .uri
            .as_deref()
            .and_then(uri_path)
            .is_some_and(|exchange_path| exchange_path.contains(path))
        {
            return false;
        }
    }

    if let Some(status) = filter.status {
        if exchange.summary.status != Some(status) {
            return false;
        }
    }

    if let Some(body_text) = filter.body_text.as_deref() {
        let request_matches = exchange
            .request_body
            .as_deref()
            .is_some_and(|body| body.contains(body_text));
        let response_matches = exchange
            .response_body
            .as_deref()
            .is_some_and(|body| body.contains(body_text));
        if !request_matches && !response_matches {
            return false;
        }
    }

    if filter.header_name.is_some() || filter.header_value.is_some() {
        let header_name = filter
            .header_name
            .as_deref()
            .map(|name| name.to_ascii_lowercase());
        let header_value = filter.header_value.as_deref();

        let matches = exchange.headers.iter().any(|(name, value)| {
            let name_matches = header_name
                .as_deref()
                .is_none_or(|needle| name.eq_ignore_ascii_case(needle));
            let value_matches = header_value.is_none_or(|needle| value.contains(needle));
            name_matches && value_matches
        });

        if !matches {
            return false;
        }
    }

    true
}

fn uri_path(uri: &str) -> Option<String> {
    uri.parse::<http::Uri>()
        .ok()
        .map(|uri| uri.path().to_string())
}

fn header_pairs(headers: Option<&Value>) -> Vec<(String, String)> {
    let Some(headers) = headers.and_then(Value::as_object) else {
        return Vec::new();
    };

    headers
        .iter()
        .filter_map(|(name, value)| {
            header_value_to_string(value).map(|value| (name.to_string(), value))
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::events::{
        BodyChunkEvent, BodyPreviewEncoding, CapturedBodySummary, CoreEvent, CoreEventEnvelope,
        HeaderEntry, RequestHeadEvent, ResponseFinishedEvent, ResponseHeadEvent,
    };
    use crate::storage::SessionEventStore;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Default)]
    struct FakeSessionStore {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl FakeSessionStore {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls lock poisoned").clone()
        }

        fn record(&self, call: impl Into<String>) {
            self.calls
                .lock()
                .expect("calls lock poisoned")
                .push(call.into());
        }
    }

    #[async_trait]
    impl SessionStore for FakeSessionStore {
        fn search_exchanges(
            &self,
            filter: &SessionExchangeFilter,
        ) -> Result<Vec<SessionExchangeSummary>, String> {
            self.record(format!(
                "search:{}",
                filter.host.as_deref().unwrap_or_default()
            ));
            Ok(vec![SessionExchangeSummary {
                exchange_id: "exchange-1".to_string(),
                method: Some("GET".to_string()),
                uri: Some("http://api.example.test/users".to_string()),
                host: Some("api.example.test".to_string()),
                status: Some(200),
                request_time: Some(1000),
                response_time: Some(1100),
                request_body_ref: None,
                response_body_ref: None,
            }])
        }

        fn load_body(&self, body_ref: &str) -> Result<String, String> {
            self.record(format!("load-body:{body_ref}"));
            Ok("lazy-body".to_string())
        }

        fn clear(&self) -> Result<usize, String> {
            self.record("clear");
            Ok(1)
        }

        fn export_har(&self) -> Result<Value, String> {
            self.record("export-har");
            Ok(json!({ "log": { "version": "1.2", "entries": [] } }))
        }

        fn import_har(&self, har: Value) -> Result<usize, String> {
            self.record(format!(
                "import-har:{}",
                har.get("log")
                    .and_then(|log| log.get("version"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            ));
            Ok(3)
        }

        async fn replay_request(
            &self,
            exchange_id: &str,
            edit: ReplayEdit,
        ) -> Result<SessionReplayResult, String> {
            self.record(format!(
                "replay:{exchange_id}:{}",
                edit.method.as_deref().unwrap_or_default()
            ));
            Ok(SessionReplayResult {
                status: 204,
                headers: HashMap::from([("x-replay".to_string(), "ok".to_string())]),
                body: "replayed".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn core_api_session_uses_typed_store_contract_without_jsonl_events() {
        let store = FakeSessionStore::default();
        let api = CoreSessionApi::new(store.clone());

        let search = api
            .search_exchanges(SearchSessionExchangesRequest {
                api_version: CORE_API_VERSION,
                filter: SessionExchangeFilter {
                    host: Some("api.example.test".to_string()),
                    ..SessionExchangeFilter::default()
                },
            })
            .expect("search failed");
        assert_eq!(search.exchanges.len(), 1);
        assert_eq!(search.exchanges[0].exchange_id, "exchange-1");

        let body = api
            .load_body(LoadSessionBodyRequest {
                api_version: CORE_API_VERSION,
                body_ref: "body-1".to_string(),
            })
            .expect("body load failed");
        assert_eq!(body.body, "lazy-body");

        let cleared = api
            .clear(ClearSessionRequest {
                api_version: CORE_API_VERSION,
            })
            .expect("clear failed");
        assert_eq!(cleared.cleared, 1);

        let exported = api
            .export_har(ExportSessionHarRequest {
                api_version: CORE_API_VERSION,
            })
            .expect("export failed");
        assert_eq!(exported.har["log"]["version"], "1.2");

        let imported = api
            .import_har(ImportSessionHarRequest {
                api_version: CORE_API_VERSION,
                har: json!({ "log": { "version": "1.2", "entries": [] } }),
            })
            .expect("import failed");
        assert_eq!(imported.imported, 3);

        let replay = api
            .replay(ReplaySessionRequest {
                api_version: CORE_API_VERSION,
                exchange_id: "exchange-1".to_string(),
                edit: ReplayEdit {
                    method: Some("PATCH".to_string()),
                    ..ReplayEdit::default()
                },
            })
            .await
            .expect("replay failed");
        assert_eq!(replay.status, 204);
        assert_eq!(replay.body, "replayed");

        assert_eq!(
            store.calls(),
            vec![
                "search:api.example.test".to_string(),
                "load-body:body-1".to_string(),
                "clear".to_string(),
                "export-har".to_string(),
                "import-har:1.2".to_string(),
                "replay:exchange-1:PATCH".to_string(),
            ]
        );
    }

    #[test]
    fn core_api_session_search_returns_typed_exchange_summaries() {
        let dir = std::env::temp_dir().join(format!("proxyman-core-api-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");

        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-1",
                    "method": "POST",
                    "uri": "http://api.example.test/users",
                    "headers": { "content-type": "application/json" },
                    "body": "{\"name\":\"proxyman\"}",
                    "time": 1000
                }
            }))
            .expect("failed to append request");
        store
            .append(&json!({
                "NewResponse": {
                    "id": "exchange-1",
                    "uri": "http://api.example.test/users",
                    "status": 201,
                    "headers": { "content-type": "application/json" },
                    "body": "{\"ok\":true}",
                    "time": 1250
                }
            }))
            .expect("failed to append response");

        let api = CoreSessionApi::new(JsonlSessionStore::new(&path));
        let response = api
            .search_exchanges(SearchSessionExchangesRequest {
                api_version: CORE_API_VERSION,
                filter: SessionExchangeFilter {
                    method: Some("POST".to_string()),
                    host: Some("api.example.test".to_string()),
                    path: None,
                    status: Some(201),
                    body_text: None,
                    header_name: None,
                    header_value: None,
                },
            })
            .expect("failed to search through core api");

        assert_eq!(response.api_version, CORE_API_VERSION);
        assert_eq!(
            response.exchanges,
            vec![SessionExchangeSummary {
                exchange_id: "exchange-1".to_string(),
                method: Some("POST".to_string()),
                uri: Some("http://api.example.test/users".to_string()),
                host: Some("api.example.test".to_string()),
                status: Some(201),
                request_time: Some(1000),
                response_time: Some(1250),
                request_body_ref: None,
                response_body_ref: None,
            }]
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn core_api_session_rejects_unsupported_api_version() {
        let dir = std::env::temp_dir().join(format!("proxyman-core-api-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        SessionEventStore::open(&path).expect("failed to open store");

        let api = CoreSessionApi::new(JsonlSessionStore::new(&path));
        let err = api
            .search_exchanges(SearchSessionExchangesRequest {
                api_version: CORE_API_VERSION + 1,
                filter: SessionExchangeFilter::default(),
            })
            .expect_err("unsupported version should fail");

        assert_eq!(
            err,
            format!("Unsupported core API version: {}", CORE_API_VERSION + 1)
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn core_api_session_search_filters_path_and_body_text_without_returning_body() {
        let dir = std::env::temp_dir().join(format!("proxyman-core-api-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");

        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-1",
                    "method": "POST",
                    "uri": "http://api.example.test/users/1",
                    "headers": {},
                    "body": "{\"name\":\"needle\"}",
                    "time": 1000
                }
            }))
            .expect("failed to append matching request");
        store
            .append(&json!({
                "NewResponse": {
                    "id": "exchange-1",
                    "uri": "http://api.example.test/users/1",
                    "status": 201,
                    "headers": {},
                    "body": "{\"ok\":true}",
                    "time": 1200
                }
            }))
            .expect("failed to append matching response");
        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-2",
                    "method": "POST",
                    "uri": "http://api.example.test/projects/1",
                    "headers": {},
                    "body": "{\"name\":\"needle\"}",
                    "time": 1300
                }
            }))
            .expect("failed to append other request");

        let api = CoreSessionApi::new(JsonlSessionStore::new(&path));
        let response = api
            .search_exchanges(SearchSessionExchangesRequest {
                api_version: CORE_API_VERSION,
                filter: SessionExchangeFilter {
                    method: Some("POST".to_string()),
                    host: Some("api.example.test".to_string()),
                    path: Some("/users".to_string()),
                    status: Some(201),
                    body_text: Some("needle".to_string()),
                    header_name: None,
                    header_value: None,
                },
            })
            .expect("failed to search through core api");

        assert_eq!(response.exchanges.len(), 1);
        assert_eq!(response.exchanges[0].exchange_id, "exchange-1");

        let json = serde_json::to_value(&response.exchanges[0]).expect("failed to serialize");
        assert!(
            json.get("body").is_none(),
            "body text must not be returned in exchange summaries"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn core_api_session_search_filters_headers_without_returning_headers() {
        let dir = std::env::temp_dir().join(format!("proxyman-core-api-{}", uuid::Uuid::new_v4()));
        let path = dir.join("events.jsonl");
        let mut store = SessionEventStore::open(&path).expect("failed to open store");

        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-1",
                    "method": "GET",
                    "uri": "http://api.example.test/users",
                    "headers": { "x-debug-token": "needle" },
                    "body": "",
                    "time": 1000
                }
            }))
            .expect("failed to append matching request");
        store
            .append(&json!({
                "NewRequest": {
                    "id": "exchange-2",
                    "method": "GET",
                    "uri": "http://api.example.test/users",
                    "headers": { "x-other": "needle" },
                    "body": "",
                    "time": 1100
                }
            }))
            .expect("failed to append other request");

        let api = CoreSessionApi::new(JsonlSessionStore::new(&path));
        let response = api
            .search_exchanges(SearchSessionExchangesRequest {
                api_version: CORE_API_VERSION,
                filter: SessionExchangeFilter {
                    method: Some("GET".to_string()),
                    host: Some("api.example.test".to_string()),
                    path: None,
                    status: None,
                    body_text: None,
                    header_name: Some("x-debug-token".to_string()),
                    header_value: Some("needle".to_string()),
                },
            })
            .expect("failed to search through core api");

        assert_eq!(response.exchanges.len(), 1);
        assert_eq!(response.exchanges[0].exchange_id, "exchange-1");

        let json = serde_json::to_value(&response.exchanges[0]).expect("failed to serialize");
        assert!(
            json.get("headers").is_none(),
            "headers must not be returned in exchange summaries"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn core_api_session_in_memory_store_summarizes_events_and_loads_bodies() {
        let store = InMemorySessionStore::default();
        store
            .apply_event(CoreEventEnvelope {
                api_version: CORE_API_VERSION,
                event: CoreEvent::RequestHead(RequestHeadEvent {
                    exchange_id: "exchange-1".to_string(),
                    timestamp: 1000,
                    method: "POST".to_string(),
                    uri: "http://api.example.test/users".to_string(),
                    version: "HTTP/1.1".to_string(),
                    headers: vec![HeaderEntry {
                        name: "content-type".to_string(),
                        value: "application/json".to_string(),
                    }],
                    captured_body: None,
                }),
            })
            .expect("request head should apply");
        store
            .apply_event(CoreEventEnvelope {
                api_version: CORE_API_VERSION,
                event: CoreEvent::RequestBodyChunk(BodyChunkEvent {
                    exchange_id: "exchange-1".to_string(),
                    timestamp: 1001,
                    uri: "http://api.example.test/users".to_string(),
                    offset: 0,
                    byte_len: 17,
                    preview: "{\"name\":\"needle\"}".to_string(),
                    preview_encoding: BodyPreviewEncoding::Utf8,
                    lossy_preview: false,
                    preview_truncated: false,
                }),
            })
            .expect("request body chunk should apply");
        store
            .apply_event(CoreEventEnvelope {
                api_version: CORE_API_VERSION,
                event: CoreEvent::ResponseHead(ResponseHeadEvent {
                    exchange_id: "exchange-1".to_string(),
                    timestamp: 1100,
                    uri: "http://api.example.test/users".to_string(),
                    status: 201,
                    version: "HTTP/1.1".to_string(),
                    headers: vec![HeaderEntry {
                        name: "content-type".to_string(),
                        value: "application/json".to_string(),
                    }],
                    captured_body: Some(CapturedBodySummary {
                        preview: "{\"ok\":true}".to_string(),
                        size: 11,
                        truncated: false,
                        body_ref: None,
                    }),
                }),
            })
            .expect("response head should apply");
        store
            .apply_event(CoreEventEnvelope {
                api_version: CORE_API_VERSION,
                event: CoreEvent::ResponseFinished(ResponseFinishedEvent {
                    exchange_id: "exchange-1".to_string(),
                    timestamp: 1200,
                    status: 201,
                    captured_body: None,
                }),
            })
            .expect("response finished should apply");

        let api = CoreSessionApi::new(store.clone());
        let response = api
            .search_exchanges(SearchSessionExchangesRequest {
                api_version: CORE_API_VERSION,
                filter: SessionExchangeFilter {
                    method: Some("POST".to_string()),
                    host: Some("api.example.test".to_string()),
                    body_text: Some("needle".to_string()),
                    ..SessionExchangeFilter::default()
                },
            })
            .expect("search should use in-memory data");

        assert_eq!(response.exchanges.len(), 1);
        let summary = &response.exchanges[0];
        assert_eq!(summary.exchange_id, "exchange-1");
        assert_eq!(summary.status, Some(201));
        assert_eq!(
            summary.request_body_ref.as_deref(),
            Some("memory://exchange-1/request")
        );
        assert_eq!(
            summary.response_body_ref.as_deref(),
            Some("memory://exchange-1/response")
        );

        let request_body = api
            .load_body(LoadSessionBodyRequest {
                api_version: CORE_API_VERSION,
                body_ref: summary.request_body_ref.clone().expect("request body ref"),
            })
            .expect("request body should load");
        assert_eq!(request_body.body, "{\"name\":\"needle\"}");

        let response_body = api
            .load_body(LoadSessionBodyRequest {
                api_version: CORE_API_VERSION,
                body_ref: summary.response_body_ref.clone().expect("response body ref"),
            })
            .expect("response body should load");
        assert_eq!(response_body.body, "{\"ok\":true}");

        let cleared = api
            .clear(ClearSessionRequest {
                api_version: CORE_API_VERSION,
            })
            .expect("clear should succeed");
        assert_eq!(cleared.cleared, 1);
        assert!(
            api.search_exchanges(SearchSessionExchangesRequest {
                api_version: CORE_API_VERSION,
                filter: SessionExchangeFilter::default(),
            })
            .expect("search after clear should succeed")
            .exchanges
            .is_empty()
        );
    }
}
