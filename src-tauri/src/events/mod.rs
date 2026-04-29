use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Uri, Version};
use hyper::{body::to_bytes, Body, Request, Response};
use serde::Serialize;
use uuid::Uuid;

use crate::processors::processor_effect::ProcessorEffects;

#[derive(Debug, Serialize, Clone)]
pub enum Events {
    NewRequest(RequestEvent),
    NewResponse(ResponseEvent),
    SseEvent(ServerSentEvent),
    WebSocketMessage(WebSocketMessageEvent),
}

impl From<RequestEvent> for Events {
    fn from(value: RequestEvent) -> Self {
        Self::NewRequest(value)
    }
}

impl From<ResponseEvent> for Events {
    fn from(value: ResponseEvent) -> Self {
        Self::NewResponse(value)
    }
}

impl From<ServerSentEvent> for Events {
    fn from(value: ServerSentEvent) -> Self {
        Self::SseEvent(value)
    }
}

impl From<WebSocketMessageEvent> for Events {
    fn from(value: WebSocketMessageEvent) -> Self {
        Self::WebSocketMessage(value)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequestEvent {
    id: Uuid,
    #[serde(with = "http_serde::method")]
    method: Method,
    #[serde(with = "http_serde::uri")]
    uri: Uri,
    #[serde(with = "http_serde::version")]
    version: Version,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: String,
    time: i64,
}

impl RequestEvent {
    pub async fn new(id: Uuid, req: &mut Request<Body>) -> Self {
        let mut body = req.body_mut();
        let body_bytes = to_bytes(&mut body).await.unwrap_or_default();
        *body = Body::from(body_bytes.clone());

        let body_str = transform_bytes_to_string(body_bytes);

        Self {
            id,
            method: req.method().clone(),
            uri: req.uri().clone(),
            version: req.version(),
            headers: req.headers().clone(),
            body: body_str,
            time: chrono::Local::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResponseEvent {
    id: Uuid,
    #[serde(with = "http_serde::uri")]
    uri: Uri,
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
    #[serde(with = "http_serde::version")]
    version: Version,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: String,
    effects: Option<ProcessorEffects>,
    time: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerSentEvent {
    id: Uuid,
    #[serde(with = "http_serde::uri")]
    uri: Uri,
    event: Option<String>,
    data: String,
    last_event_id: Option<String>,
    retry: Option<u64>,
    time: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketMessageEvent {
    id: Uuid,
    #[serde(with = "http_serde::uri")]
    uri: Uri,
    direction: String,
    opcode: String,
    payload: String,
    payload_len: usize,
    preview_encoding: String,
    lossy_preview: bool,
    preview_truncated: bool,
    close_code: Option<u16>,
    close_reason: Option<String>,
    time: i64,
}

impl WebSocketMessageEvent {
    pub(crate) fn new(
        id: Uuid,
        uri: Uri,
        direction: &'static str,
        opcode: &'static str,
        payload: String,
        payload_len: usize,
        preview_encoding: &'static str,
        lossy_preview: bool,
        preview_truncated: bool,
        close_code: Option<u16>,
        close_reason: Option<String>,
    ) -> Self {
        Self {
            id,
            uri,
            direction: direction.to_string(),
            opcode: opcode.to_string(),
            payload,
            payload_len,
            preview_encoding: preview_encoding.to_string(),
            lossy_preview,
            preview_truncated,
            close_code,
            close_reason,
            time: chrono::Local::now().timestamp_millis(),
        }
    }
}

impl ServerSentEvent {
    pub(crate) fn new(
        id: Uuid,
        uri: Uri,
        event: Option<String>,
        data: String,
        last_event_id: Option<String>,
        retry: Option<u64>,
    ) -> Self {
        Self {
            id,
            uri,
            event,
            data,
            last_event_id,
            retry,
            time: chrono::Local::now().timestamp_millis(),
        }
    }
}

impl ResponseEvent {
    pub async fn new(
        id: Uuid,
        uri: Uri,
        res: &mut Response<Body>,
        effects: Option<ProcessorEffects>,
    ) -> Self {
        let mut body = res.body_mut();
        let body_bytes = to_bytes(&mut body).await.unwrap_or_default();
        *body = Body::from(body_bytes.clone());

        let body_str = transform_bytes_to_string(body_bytes);

        Self {
            id,
            uri,
            status: res.status(),
            version: res.version(),
            headers: res.headers().clone(),
            body: body_str,
            effects,
            time: chrono::Local::now().timestamp_millis(),
        }
    }

    pub(crate) fn from_body_bytes(
        id: Uuid,
        uri: Uri,
        status: StatusCode,
        version: Version,
        headers: HeaderMap,
        body_bytes: Bytes,
        effects: Option<ProcessorEffects>,
    ) -> Self {
        Self {
            id,
            uri,
            status,
            version,
            headers,
            body: transform_bytes_to_string(body_bytes),
            effects,
            time: chrono::Local::now().timestamp_millis(),
        }
    }
}

fn transform_bytes_to_string(bytes: Bytes) -> String {
    String::from_utf8(bytes.into())
        .map_err(|non_utf8| String::from_utf8_lossy(non_utf8.as_bytes()).into_owned())
        .unwrap_or_else(|_| "parsing failed".into())
}
