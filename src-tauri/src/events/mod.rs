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
}

fn transform_bytes_to_string(bytes: Bytes) -> String {
    String::from_utf8(bytes.into())
        .map_err(|non_utf8| String::from_utf8_lossy(non_utf8.as_bytes()).into_owned())
        .unwrap_or_else(|_| "parsing failed".into())
}
