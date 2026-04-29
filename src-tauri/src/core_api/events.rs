use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc::{error::TrySendError, Sender};

use crate::{core_api::CORE_API_VERSION, events::Events};

const BODY_CHUNK_PREVIEW_LIMIT_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoreEventEnvelope {
    pub(crate) api_version: u16,
    pub(crate) event: CoreEvent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TypedEventBackpressurePolicy {
    DropNewest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TypedEventDropReason {
    ChannelFull,
    ChannelClosed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum TypedEventSendStatus {
    Sent,
    Dropped {
        policy: TypedEventBackpressurePolicy,
        reason: TypedEventDropReason,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub(crate) enum CoreEvent {
    ExchangeStarted(ExchangeStartedEvent),
    RequestHead(RequestHeadEvent),
    RequestBodyChunk(BodyChunkEvent),
    RequestFinished(RequestFinishedEvent),
    ResponseHead(ResponseHeadEvent),
    ResponseBodyChunk(BodyChunkEvent),
    ResponseFinished(ResponseFinishedEvent),
    ExchangeFinished(ExchangeFinishedEvent),
    ExchangeError(ExchangeErrorEvent),
    SseEvent(SseEvent),
    WebSocketMessage(WebSocketMessageEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExchangeStartedEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) method: String,
    pub(crate) uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestHeadEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) method: String,
    pub(crate) uri: String,
    pub(crate) version: String,
    pub(crate) headers: Vec<HeaderEntry>,
    pub(crate) captured_body: Option<CapturedBodySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestFinishedEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) captured_body: Option<CapturedBodySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BodyChunkEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) uri: String,
    pub(crate) offset: usize,
    pub(crate) byte_len: usize,
    pub(crate) preview: String,
    pub(crate) preview_encoding: BodyPreviewEncoding,
    pub(crate) lossy_preview: bool,
    pub(crate) preview_truncated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum BodyPreviewEncoding {
    Utf8,
    Utf8Lossy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResponseHeadEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) uri: String,
    pub(crate) status: u16,
    pub(crate) version: String,
    pub(crate) headers: Vec<HeaderEntry>,
    pub(crate) captured_body: Option<CapturedBodySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResponseFinishedEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) status: u16,
    pub(crate) captured_body: Option<CapturedBodySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExchangeFinishedEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) status: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExchangeErrorEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) uri: Option<String>,
    pub(crate) phase: ExchangeErrorPhase,
    pub(crate) message: String,
    pub(crate) recoverable: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ExchangeErrorPhase {
    RequestDecode,
    ResponseDecode,
    Upstream,
    WebSocket,
    Tunnel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SseEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) uri: String,
    pub(crate) event: Option<String>,
    pub(crate) data: String,
    pub(crate) last_event_id: Option<String>,
    pub(crate) retry: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebSocketMessageEvent {
    pub(crate) exchange_id: String,
    pub(crate) timestamp: i64,
    pub(crate) uri: String,
    pub(crate) direction: String,
    pub(crate) opcode: String,
    pub(crate) payload_preview: String,
    pub(crate) payload_len: usize,
    pub(crate) preview_encoding: BodyPreviewEncoding,
    pub(crate) lossy_preview: bool,
    pub(crate) preview_truncated: bool,
    pub(crate) close_code: Option<u16>,
    pub(crate) close_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HeaderEntry {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapturedBodySummary {
    pub(crate) preview: String,
    pub(crate) size: usize,
    pub(crate) truncated: bool,
    pub(crate) body_ref: Option<String>,
}

pub(crate) fn legacy_event_to_core_events(
    event: &Events,
) -> Result<Vec<CoreEventEnvelope>, String> {
    let value = serde_json::to_value(event).map_err(|err| {
        log::error!("Serialize legacy proxy event for core API failed: {err}");
        "Serialize legacy proxy event for core API failed".to_string()
    })?;

    legacy_event_value_to_core_events(&value)
}

pub(crate) fn typed_event_backpressure_policy() -> TypedEventBackpressurePolicy {
    TypedEventBackpressurePolicy::DropNewest
}

pub(crate) fn try_send_typed_event(
    sender: &Sender<CoreEventEnvelope>,
    event: CoreEventEnvelope,
) -> TypedEventSendStatus {
    match sender.try_send(event) {
        Ok(()) => TypedEventSendStatus::Sent,
        Err(TrySendError::Full(_)) => TypedEventSendStatus::Dropped {
            policy: typed_event_backpressure_policy(),
            reason: TypedEventDropReason::ChannelFull,
        },
        Err(TrySendError::Closed(_)) => TypedEventSendStatus::Dropped {
            policy: typed_event_backpressure_policy(),
            reason: TypedEventDropReason::ChannelClosed,
        },
    }
}

pub(crate) fn legacy_event_value_to_core_event(
    value: &Value,
) -> Result<CoreEventEnvelope, String> {
    let event = if let Some(request) = value.get("NewRequest") {
        CoreEvent::RequestHead(request_head_from_value(request)?)
    } else if let Some(response) = value.get("NewResponse") {
        CoreEvent::ResponseHead(response_head_from_value(response)?)
    } else if let Some(sse) = value.get("SseEvent") {
        CoreEvent::SseEvent(sse_event_from_value(sse)?)
    } else if let Some(message) = value.get("WebSocketMessage") {
        CoreEvent::WebSocketMessage(websocket_message_from_value(message)?)
    } else {
        return Err("Unsupported legacy proxy event".to_string());
    };

    Ok(CoreEventEnvelope {
        api_version: CORE_API_VERSION,
        event,
    })
}

pub(crate) fn legacy_event_value_to_core_events(
    value: &Value,
) -> Result<Vec<CoreEventEnvelope>, String> {
    if let Some(request) = value.get("NewRequest") {
        let mut events = vec![
            envelope(CoreEvent::ExchangeStarted(exchange_started_from_value(request)?)),
            envelope(CoreEvent::RequestHead(request_head_from_value(request)?)),
        ];
        if let Some(chunk) = request_body_chunk_from_value(request)? {
            events.push(envelope(CoreEvent::RequestBodyChunk(chunk)));
        }
        events.push(envelope(CoreEvent::RequestFinished(
            request_finished_from_value(request)?,
        )));

        return Ok(events);
    }

    if let Some(response) = value.get("NewResponse") {
        return Ok(vec![
            envelope(CoreEvent::ResponseHead(response_head_from_value(response)?)),
            envelope(CoreEvent::ResponseFinished(response_finished_from_value(response)?)),
            envelope(CoreEvent::ExchangeFinished(exchange_finished_from_response_value(
                response,
            )?)),
        ]);
    }

    Ok(vec![legacy_event_value_to_core_event(value)?])
}

pub(crate) fn response_body_chunk_event(
    exchange_id: String,
    uri: String,
    offset: usize,
    chunk: &[u8],
) -> CoreEventEnvelope {
    envelope(CoreEvent::ResponseBodyChunk(body_chunk_event(
        exchange_id,
        uri,
        offset,
        chunk,
    )))
}

pub(crate) fn exchange_error_event(
    exchange_id: String,
    uri: Option<String>,
    phase: ExchangeErrorPhase,
    message: String,
    recoverable: bool,
) -> CoreEventEnvelope {
    envelope(CoreEvent::ExchangeError(ExchangeErrorEvent {
        exchange_id,
        timestamp: chrono::Local::now().timestamp_millis(),
        uri,
        phase,
        message,
        recoverable,
    }))
}

fn envelope(event: CoreEvent) -> CoreEventEnvelope {
    CoreEventEnvelope {
        api_version: CORE_API_VERSION,
        event,
    }
}

fn body_chunk_event(
    exchange_id: String,
    uri: String,
    offset: usize,
    chunk: &[u8],
) -> BodyChunkEvent {
    let preview_len = chunk.len().min(BODY_CHUNK_PREVIEW_LIMIT_BYTES);

    let preview_bytes = &chunk[..preview_len];
    let (preview, preview_encoding, lossy_preview) = match std::str::from_utf8(preview_bytes) {
        Ok(preview) => (preview.to_string(), BodyPreviewEncoding::Utf8, false),
        Err(_) => (
            String::from_utf8_lossy(preview_bytes).into_owned(),
            BodyPreviewEncoding::Utf8Lossy,
            true,
        ),
    };

    BodyChunkEvent {
        exchange_id,
        timestamp: chrono::Local::now().timestamp_millis(),
        uri,
        offset,
        byte_len: chunk.len(),
        preview,
        preview_encoding,
        lossy_preview,
        preview_truncated: chunk.len() > BODY_CHUNK_PREVIEW_LIMIT_BYTES,
    }
}

fn exchange_started_from_value(value: &Value) -> Result<ExchangeStartedEvent, String> {
    Ok(ExchangeStartedEvent {
        exchange_id: required_string(value, "id")?,
        timestamp: required_i64(value, "time")?,
        method: required_string(value, "method")?,
        uri: required_string(value, "uri")?,
    })
}

fn request_head_from_value(value: &Value) -> Result<RequestHeadEvent, String> {
    Ok(RequestHeadEvent {
        exchange_id: required_string(value, "id")?,
        timestamp: required_i64(value, "time")?,
        method: required_string(value, "method")?,
        uri: required_string(value, "uri")?,
        version: required_string(value, "version")?,
        headers: header_entries(value.get("headers")),
        captured_body: captured_body_summary(value),
    })
}

fn request_finished_from_value(value: &Value) -> Result<RequestFinishedEvent, String> {
    Ok(RequestFinishedEvent {
        exchange_id: required_string(value, "id")?,
        timestamp: required_i64(value, "time")?,
        captured_body: captured_body_summary(value),
    })
}

fn request_body_chunk_from_value(value: &Value) -> Result<Option<BodyChunkEvent>, String> {
    let Some(body) = value.get("body").and_then(Value::as_str) else {
        return Ok(None);
    };
    if body.is_empty() {
        return Ok(None);
    }

    Ok(Some(body_chunk_event(
        required_string(value, "id")?,
        required_string(value, "uri")?,
        0,
        body.as_bytes(),
    )))
}

fn response_head_from_value(value: &Value) -> Result<ResponseHeadEvent, String> {
    Ok(ResponseHeadEvent {
        exchange_id: required_string(value, "id")?,
        timestamp: required_i64(value, "time")?,
        uri: required_string(value, "uri")?,
        status: value
            .get("status")
            .and_then(Value::as_u64)
            .and_then(|status| u16::try_from(status).ok())
            .ok_or_else(|| "Legacy response event missing status".to_string())?,
        version: required_string(value, "version")?,
        headers: header_entries(value.get("headers")),
        captured_body: captured_body_summary(value),
    })
}

fn response_finished_from_value(value: &Value) -> Result<ResponseFinishedEvent, String> {
    Ok(ResponseFinishedEvent {
        exchange_id: required_string(value, "id")?,
        timestamp: required_i64(value, "time")?,
        status: response_status(value)?,
        captured_body: captured_body_summary(value),
    })
}

fn exchange_finished_from_response_value(value: &Value) -> Result<ExchangeFinishedEvent, String> {
    Ok(ExchangeFinishedEvent {
        exchange_id: required_string(value, "id")?,
        timestamp: required_i64(value, "time")?,
        status: Some(response_status(value)?),
    })
}

fn response_status(value: &Value) -> Result<u16, String> {
    value
        .get("status")
        .and_then(Value::as_u64)
        .and_then(|status| u16::try_from(status).ok())
        .ok_or_else(|| "Legacy response event missing status".to_string())
}

fn sse_event_from_value(value: &Value) -> Result<SseEvent, String> {
    Ok(SseEvent {
        exchange_id: required_string(value, "id")?,
        timestamp: required_i64(value, "time")?,
        uri: required_string(value, "uri")?,
        event: optional_string(value, "event"),
        data: required_string(value, "data")?,
        last_event_id: optional_string(value, "lastEventId"),
        retry: value.get("retry").and_then(Value::as_u64),
    })
}

fn websocket_message_from_value(value: &Value) -> Result<WebSocketMessageEvent, String> {
    Ok(WebSocketMessageEvent {
        exchange_id: required_string(value, "id")?,
        timestamp: required_i64(value, "time")?,
        uri: required_string(value, "uri")?,
        direction: required_string(value, "direction")?,
        opcode: required_string(value, "opcode")?,
        payload_preview: required_string(value, "payload")?,
        payload_len: value
            .get("payloadLen")
            .and_then(Value::as_u64)
            .and_then(|len| usize::try_from(len).ok())
            .ok_or_else(|| "Legacy WebSocket event missing payloadLen".to_string())?,
        preview_encoding: optional_preview_encoding(value, "previewEncoding")?
            .unwrap_or(BodyPreviewEncoding::Utf8),
        lossy_preview: value
            .get("lossyPreview")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        preview_truncated: value
            .get("previewTruncated")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        close_code: value
            .get("closeCode")
            .and_then(Value::as_u64)
            .and_then(|code| u16::try_from(code).ok()),
        close_reason: optional_string(value, "closeReason"),
    })
}

fn optional_preview_encoding(
    value: &Value,
    field: &str,
) -> Result<Option<BodyPreviewEncoding>, String> {
    let Some(encoding) = value.get(field).and_then(Value::as_str) else {
        return Ok(None);
    };

    match encoding {
        "utf8" => Ok(Some(BodyPreviewEncoding::Utf8)),
        "utf8Lossy" => Ok(Some(BodyPreviewEncoding::Utf8Lossy)),
        other => Err(format!("Unsupported preview encoding: {other}")),
    }
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("Legacy proxy event missing {field}"))
}

fn optional_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn required_i64(value: &Value, field: &str) -> Result<i64, String> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("Legacy proxy event missing {field}"))
}

fn header_entries(headers: Option<&Value>) -> Vec<HeaderEntry> {
    let Some(headers) = headers.and_then(Value::as_object) else {
        return Vec::new();
    };

    let mut entries = headers
        .iter()
        .filter_map(|(name, value)| {
            header_value_to_string(value).map(|value| HeaderEntry {
                name: name.clone(),
                value,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
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

fn captured_body_summary(value: &Value) -> Option<CapturedBodySummary> {
    let preview = value.get("body").and_then(Value::as_str)?.to_string();
    let size = value
        .get("bodySize")
        .and_then(Value::as_u64)
        .and_then(|size| usize::try_from(size).ok())
        .unwrap_or(preview.len());
    let truncated = value
        .get("bodyTruncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let body_ref = optional_string(value, "bodyRef");

    Some(CapturedBodySummary {
        preview,
        size,
        truncated,
        body_ref,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::CORE_API_VERSION;
    use serde_json::json;

    #[test]
    fn core_api_events_maps_legacy_request_to_typed_request_head() {
        let event = legacy_event_value_to_core_event(&json!({
            "NewRequest": {
                "id": "exchange-1",
                "method": "POST",
                "uri": "http://api.example.test/users",
                "version": "HTTP/1.1",
                "headers": {
                    "content-type": "application/json",
                    "accept": ["application/json", "text/plain"]
                },
                "body": "{\"name\":\"proxyman\"}",
                "bodyRef": "body-1",
                "bodySize": 70000,
                "bodyTruncated": true,
                "time": 1000
            }
        }))
        .expect("failed to map request event");

        assert_eq!(
            event,
            CoreEventEnvelope {
                api_version: CORE_API_VERSION,
                event: CoreEvent::RequestHead(RequestHeadEvent {
                    exchange_id: "exchange-1".to_string(),
                    timestamp: 1000,
                    method: "POST".to_string(),
                    uri: "http://api.example.test/users".to_string(),
                    version: "HTTP/1.1".to_string(),
                    headers: vec![
                        HeaderEntry {
                            name: "accept".to_string(),
                            value: "application/json, text/plain".to_string(),
                        },
                        HeaderEntry {
                            name: "content-type".to_string(),
                            value: "application/json".to_string(),
                        },
                    ],
                    captured_body: Some(CapturedBodySummary {
                        preview: "{\"name\":\"proxyman\"}".to_string(),
                        size: 70000,
                        truncated: true,
                        body_ref: Some("body-1".to_string()),
                    }),
                }),
            }
        );
    }

    #[test]
    fn core_api_events_maps_legacy_response_to_typed_response_head() {
        let event = legacy_event_value_to_core_event(&json!({
            "NewResponse": {
                "id": "exchange-1",
                "uri": "http://api.example.test/users",
                "status": 201,
                "version": "HTTP/1.1",
                "headers": {
                    "content-type": "application/json"
                },
                "body": "{\"ok\":true}",
                "time": 1250
            }
        }))
        .expect("failed to map response event");

        assert_eq!(
            event,
            CoreEventEnvelope {
                api_version: CORE_API_VERSION,
                event: CoreEvent::ResponseHead(ResponseHeadEvent {
                    exchange_id: "exchange-1".to_string(),
                    timestamp: 1250,
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
            }
        );
    }

    #[test]
    fn core_api_events_expands_request_lifecycle_events() {
        let events = legacy_event_value_to_core_events(&json!({
            "NewRequest": {
                "id": "exchange-1",
                "method": "GET",
                "uri": "http://api.example.test/users",
                "version": "HTTP/1.1",
                "headers": {},
                "body": "",
                "time": 1000
            }
        }))
        .expect("failed to expand request lifecycle events");

        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[0].event,
            CoreEvent::ExchangeStarted(ExchangeStartedEvent { .. })
        ));
        assert!(matches!(events[1].event, CoreEvent::RequestHead(_)));
        assert!(matches!(
            events[2].event,
            CoreEvent::RequestFinished(RequestFinishedEvent { .. })
        ));
    }

    #[test]
    fn core_api_events_expands_request_body_chunk_event() {
        let events = legacy_event_value_to_core_events(&json!({
            "NewRequest": {
                "id": "exchange-1",
                "method": "POST",
                "uri": "http://api.example.test/users",
                "version": "HTTP/1.1",
                "headers": {},
                "body": "hello",
                "time": 1000
            }
        }))
        .expect("failed to expand request lifecycle events");

        assert_eq!(events.len(), 4);
        match &events[2].event {
            CoreEvent::RequestBodyChunk(chunk) => {
                assert_eq!(chunk.exchange_id, "exchange-1");
                assert_eq!(chunk.uri, "http://api.example.test/users");
                assert_eq!(chunk.offset, 0);
                assert_eq!(chunk.byte_len, 5);
                assert_eq!(chunk.preview, "hello");
                assert_eq!(chunk.preview_encoding, BodyPreviewEncoding::Utf8);
                assert!(!chunk.lossy_preview);
            }
            other => panic!("unexpected event: {other:?}"),
        }
        assert!(matches!(
            events[3].event,
            CoreEvent::RequestFinished(RequestFinishedEvent { .. })
        ));
    }

    #[test]
    fn core_api_events_builds_exchange_error_event() {
        let event = exchange_error_event(
            "exchange-1".to_string(),
            Some("http://api.example.test/users".to_string()),
            ExchangeErrorPhase::RequestDecode,
            "unsupported encoding".to_string(),
            false,
        );

        assert_eq!(event.api_version, CORE_API_VERSION);
        match event.event {
            CoreEvent::ExchangeError(error) => {
                assert_eq!(error.exchange_id, "exchange-1");
                assert_eq!(error.uri.as_deref(), Some("http://api.example.test/users"));
                assert_eq!(error.phase, ExchangeErrorPhase::RequestDecode);
                assert_eq!(error.message, "unsupported encoding");
                assert!(!error.recoverable);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn core_api_events_expands_response_lifecycle_events() {
        let events = legacy_event_value_to_core_events(&json!({
            "NewResponse": {
                "id": "exchange-1",
                "uri": "http://api.example.test/users",
                "status": 200,
                "version": "HTTP/1.1",
                "headers": {},
                "body": "ok",
                "time": 1200
            }
        }))
        .expect("failed to expand response lifecycle events");

        assert_eq!(events.len(), 3);
        assert!(matches!(events[0].event, CoreEvent::ResponseHead(_)));
        assert!(matches!(
            events[1].event,
            CoreEvent::ResponseFinished(ResponseFinishedEvent { status: 200, .. })
        ));
        assert!(matches!(
            events[2].event,
            CoreEvent::ExchangeFinished(ExchangeFinishedEvent {
                status: Some(200),
                ..
            })
        ));
    }

    #[test]
    fn core_api_events_builds_response_body_chunk_event() {
        let event = response_body_chunk_event(
            "exchange-1".to_string(),
            "http://api.example.test/users".to_string(),
            5,
            b"hello",
        );

        assert_eq!(event.api_version, CORE_API_VERSION);
        match event.event {
            CoreEvent::ResponseBodyChunk(chunk) => {
                assert_eq!(chunk.exchange_id, "exchange-1");
                assert_eq!(chunk.uri, "http://api.example.test/users");
                assert_eq!(chunk.offset, 5);
                assert_eq!(chunk.byte_len, 5);
                assert_eq!(chunk.preview, "hello");
                assert_eq!(chunk.preview_encoding, BodyPreviewEncoding::Utf8);
                assert!(!chunk.lossy_preview);
                assert!(!chunk.preview_truncated);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn core_api_events_truncates_body_chunk_preview() {
        let chunk = vec![b'x'; BODY_CHUNK_PREVIEW_LIMIT_BYTES + 1];
        let event = response_body_chunk_event(
            "exchange-1".to_string(),
            "http://api.example.test/users".to_string(),
            0,
            &chunk,
        );

        match event.event {
            CoreEvent::ResponseBodyChunk(chunk) => {
                assert_eq!(chunk.byte_len, BODY_CHUNK_PREVIEW_LIMIT_BYTES + 1);
                assert_eq!(chunk.preview.len(), BODY_CHUNK_PREVIEW_LIMIT_BYTES);
                assert!(chunk.preview_truncated);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn core_api_events_marks_lossy_binary_body_chunk_preview() {
        let event = response_body_chunk_event(
            "exchange-1".to_string(),
            "http://api.example.test/bin".to_string(),
            0,
            &[0xff, 0xfe, b'a'],
        );

        match event.event {
            CoreEvent::ResponseBodyChunk(chunk) => {
                assert_eq!(chunk.byte_len, 3);
                assert_eq!(chunk.preview_encoding, BodyPreviewEncoding::Utf8Lossy);
                assert!(chunk.lossy_preview);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn core_api_events_reports_drop_newest_backpressure_when_channel_full() {
        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        let first = response_body_chunk_event(
            "exchange-1".to_string(),
            "http://api.example.test/bin".to_string(),
            0,
            b"first",
        );
        let second = response_body_chunk_event(
            "exchange-1".to_string(),
            "http://api.example.test/bin".to_string(),
            5,
            b"second",
        );

        assert_eq!(try_send_typed_event(&sender, first), TypedEventSendStatus::Sent);
        assert_eq!(
            try_send_typed_event(&sender, second),
            TypedEventSendStatus::Dropped {
                policy: TypedEventBackpressurePolicy::DropNewest,
                reason: TypedEventDropReason::ChannelFull,
            }
        );
    }

    #[test]
    fn core_api_events_maps_websocket_binary_preview_metadata() {
        let event = legacy_event_value_to_core_event(&json!({
            "WebSocketMessage": {
                "id": "exchange-1",
                "uri": "ws://api.example.test/socket",
                "direction": "clientToServer",
                "opcode": "binary",
                "payload": "\u{fffd}\u{fffd}a",
                "payloadLen": 3,
                "previewEncoding": "utf8Lossy",
                "lossyPreview": true,
                "previewTruncated": false,
                "closeCode": null,
                "closeReason": null,
                "time": 2000
            }
        }))
        .expect("failed to map websocket event");

        assert_eq!(event.api_version, CORE_API_VERSION);
        match event.event {
            CoreEvent::WebSocketMessage(message) => {
                assert_eq!(message.exchange_id, "exchange-1");
                assert_eq!(message.opcode, "binary");
                assert_eq!(message.payload_preview, "\u{fffd}\u{fffd}a");
                assert_eq!(message.payload_len, 3);
                assert_eq!(message.preview_encoding, BodyPreviewEncoding::Utf8Lossy);
                assert!(message.lossy_preview);
                assert!(!message.preview_truncated);
                assert_eq!(message.close_code, None);
                assert_eq!(message.close_reason, None);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn core_api_events_maps_websocket_close_metadata() {
        let event = legacy_event_value_to_core_event(&json!({
            "WebSocketMessage": {
                "id": "exchange-1",
                "uri": "ws://api.example.test/socket",
                "direction": "serverToClient",
                "opcode": "close",
                "payload": "done",
                "payloadLen": 6,
                "previewEncoding": "utf8",
                "lossyPreview": false,
                "previewTruncated": false,
                "closeCode": 1000,
                "closeReason": "done",
                "time": 2100
            }
        }))
        .expect("failed to map websocket event");

        match event.event {
            CoreEvent::WebSocketMessage(message) => {
                assert_eq!(message.opcode, "close");
                assert_eq!(message.payload_preview, "done");
                assert_eq!(message.payload_len, 6);
                assert_eq!(message.preview_encoding, BodyPreviewEncoding::Utf8);
                assert!(!message.lossy_preview);
                assert!(!message.preview_truncated);
                assert_eq!(message.close_code, Some(1000));
                assert_eq!(message.close_reason.as_deref(), Some("done"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
