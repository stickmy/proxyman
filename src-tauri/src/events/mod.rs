use std::{str, string};
use std::collections::HashMap;

use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Uri, Version};
use hyper::{Body, body::to_bytes, Request, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::proxy::processor::RuleHit;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    hit_rules: Option<RuleHit>,
    time: i64,
}

impl ResponseEvent {
    pub async fn new(
        id: Uuid,
        uri: Uri,
        res: &mut Response<Body>,
        hit_rules: Option<RuleHit>,
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
            hit_rules,
            time: chrono::Local::now().timestamp_millis(),
        }
    }
}

trait ToString {
    fn to_string(&self) -> String;
}

trait ToHashMap {
    fn to_hash_map(&self) -> HashMap<String, String>;
}

impl ToString for Version {
    fn to_string(&self) -> String {
        match *self {
            Version::HTTP_2 => string::ToString::to_string("HTTP_2"),
            Version::HTTP_3 => string::ToString::to_string("HTTP_3"),
            Version::HTTP_09 => string::ToString::to_string("HTTP_09"),
            Version::HTTP_10 => string::ToString::to_string("HTTP_10"),
            Version::HTTP_11 => string::ToString::to_string("HTTP_11"),
            _ => string::ToString::to_string("Unrecognized"),
        }
    }
}

impl ToHashMap for HeaderMap {
    fn to_hash_map(&self) -> HashMap<String, String> {
        let mut headers: HashMap<String, String> = HashMap::new();

        for (k, v) in self.iter() {
            headers.insert(string::ToString::to_string(k), string::ToString::to_string(v.to_str().unwrap()));
        }

        headers
    }
}

fn transform_bytes_to_string(bytes: Bytes) -> String {
    String::from_utf8(bytes.into())
        .map_err(|non_utf8| String::from_utf8_lossy(non_utf8.as_bytes()).into_owned()).unwrap_or_else(|_| "parsing failed".into())
}
