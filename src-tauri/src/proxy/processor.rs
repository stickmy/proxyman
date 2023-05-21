use async_trait::async_trait;
use http::StatusCode;
use hyper::{Body, Request, Response};
use tokio_tungstenite::tungstenite::Message;

use crate::error::Error;

#[derive(Debug)]
pub struct RequestOrResponse {
    pub req: Request<Body>,
    pub res: Option<Response<Body>>,
    pub hit_rules: Option<Vec<String>>,
}

impl From<Request<Body>> for RequestOrResponse {
    fn from(value: Request<Body>) -> Self {
        Self {
            req: value,
            res: None,
            hit_rules: None,
        }
    }
}

impl From<(Request<Body>, Response<Body>)> for RequestOrResponse {
    fn from(value: (Request<Body>, Response<Body>)) -> Self {
        Self {
            req: value.0,
            res: Some(value.1),
            hit_rules: None,
        }
    }
}

#[async_trait]
pub trait HttpProcessor: Clone + Send + Sync + 'static {
    async fn process_request(&self, req: Request<Body>) -> RequestOrResponse {
        req.into()
    }

    async fn process_response(&self, res: Response<Body>) -> Response<Body> {
        res
    }

    async fn process_error(&self, err: Error) -> Response<Body> {
        log::error!("Failed to tunnel request: {}", err);

        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::empty())
            .expect("Failed to build response")
    }
}

#[async_trait]
pub trait WebsocketProcessor: Clone + Send + Sync + 'static {
    async fn process_message(&self, message: Message) -> Option<Message> {
        Some(message)
    }
}
