use std::{convert::Infallible, sync::Arc};

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use http::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    uri::{Authority, Scheme},
    HeaderMap, Method, StatusCode, Uri,
};
use hyper::{
    client::connect::Connect, header::Entry, server::conn::Http, service::service_fn,
    upgrade::Upgraded, Body, Client, Request, Response,
};
use snafu::ResultExt;
use tauri::async_runtime::Mutex;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    net::TcpStream,
    sync::mpsc::Sender,
};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::{tungstenite, Connector, WebSocketStream};
use uuid::Uuid;

use crate::{
    ca::CertificateAuthority,
    core_api::events::{self as core_events, CoreEventEnvelope, ExchangeErrorPhase},
    error::{
        endpoint_error::{EndpointError, HttpError, WebsocketProtocolError},
        ClientError, ServerError,
    },
    events::{Events, RequestEvent, ResponseEvent, ServerSentEvent, WebSocketMessageEvent},
};

use super::decoder::{decode_request, decode_response};
use super::rewind::Rewind;
use super::sse::SseParser;
use crate::processors::processor;

const BODY_CAPTURE_LIMIT_BYTES: usize = 64 * 1024;
const WEBSOCKET_MESSAGE_PREVIEW_LIMIT_BYTES: usize = 16 * 1024;

pub struct Tunnel<CA, C, P> {
    pub ca: Arc<CA>,
    pub client: Client<C>,
    pub websocket_connector: Option<Connector>,
    pub transporter: Sender<Events>,
    pub typed_transporter: Option<Sender<CoreEventEnvelope>>,
    pub processor: Arc<Mutex<P>>,
}

impl<CA, C, P> Clone for Tunnel<CA, C, P>
where
    C: Clone,
    P: Clone,
{
    fn clone(&self) -> Self {
        Tunnel {
            ca: Arc::clone(&self.ca),
            client: self.client.clone(),
            websocket_connector: self.websocket_connector.clone(),
            transporter: self.transporter.clone(),
            typed_transporter: self.typed_transporter.clone(),
            processor: Arc::clone(&self.processor),
        }
    }
}

impl<CA, C, P> Tunnel<CA, C, P>
where
    CA: CertificateAuthority,
    C: Connect + Clone + Send + Sync + 'static,
    P: processor::HttpProcessor + std::fmt::Debug,
{
    async fn send_event(&self, event: Events) {
        if let Err(e) = self.transporter.send(event).await {
            log::error!("send events to client failed: {e}");
        }
    }

    pub(crate) async fn accept(self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        // 模拟 Server, 对 Client 连接
        if req.method() == Method::CONNECT {
            Ok(self.handle_connect(req))
        } else if hyper_tungstenite::is_upgrade_request(&req) {
            Ok(self.upgrade_websocket(req))
        } else {
            let conn_id = Uuid::new_v4();
            log::trace!("accept request from client: {}, {:?}", conn_id, req);
            let request_uri_before_decode = req.uri().to_string();

            let mut req = match decode_request(req).context(ClientError {
                scenario: "decoding request body failed",
            }) {
                Ok(req) => req,
                Err(err) => {
                    log::debug!("decode request failed: {err}");
                    self.send_typed_event(core_events::exchange_error_event(
                        conn_id.to_string(),
                        Some(request_uri_before_decode),
                        ExchangeErrorPhase::RequestDecode,
                        err.to_string(),
                        false,
                    ));
                    return Ok(bad_request());
                }
            };

            self.send_event(RequestEvent::new(conn_id, &mut req).await.into())
                .await;

            let processor = self.processor.lock().await.clone();
            let req_or_res = processor.process_request(req).await;

            let req = match req_or_res.res {
                Some(mut res) => {
                    self.send_event(
                        ResponseEvent::new(
                            conn_id,
                            req_or_res.req.uri().to_owned(),
                            &mut res,
                            req_or_res.processor_effects,
                        )
                        .await
                        .into(),
                    )
                    .await;
                    return Ok(res);
                }
                None => req_or_res.req,
            };

            let req_uri = req.uri().clone();

            log::trace!("send network request: {}, {:?}", conn_id, req);
            let res = self
                .client
                .request(normalize_request(req))
                .await
                .context(HttpError {})
                .context(ServerError {
                    scenario: "sending request",
                });
            log::trace!("send network request done: {}, {:?}", conn_id, res);

            let processor = self.processor.lock().await.clone();
            let mut res = match res {
                Ok(res) => res,
                Err(e) => processor.process_error(e).await,
            };

            res = match decode_response(res) {
                Ok(res) => res,
                Err(err) => {
                    log::debug!("decode response failed: {err}");
                    self.send_typed_event(core_events::exchange_error_event(
                        conn_id.to_string(),
                        Some(req_uri.to_string()),
                        ExchangeErrorPhase::ResponseDecode,
                        err.to_string(),
                        false,
                    ));
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(Body::empty())
                        .expect("Failed to build response"));
                }
            };

            let processor = self.processor.lock().await.clone();
            res = processor.process_response(&req_uri, res).await;

            Ok(tap_response_body_for_event(
                conn_id,
                req_uri,
                res,
                req_or_res.processor_effects,
                self.transporter.clone(),
                self.typed_transporter.clone(),
            ))
        }
    }

    fn handle_connect(self, mut req: Request<Body>) -> Response<Body> {
        match req.uri().authority().cloned() {
            Some(authority) => {
                let fut = async move {
                    match hyper::upgrade::on(&mut req).await {
                        Ok(mut upgraded) => {
                            let mut buffer = [0_u8; 4];
                            let bytes_read = match upgraded.read(&mut buffer).await {
                                Ok(bytes) => bytes,
                                Err(e) => {
                                    log::error!("Failed to read from upgraded connections: {e}");
                                    return;
                                }
                            };

                            let mut upgraded = Rewind::new_buffered(
                                upgraded,
                                Bytes::copy_from_slice(buffer[..bytes_read].as_ref()),
                            );

                            if &buffer == b"GET " {
                                if let Err(e) =
                                    self.serve_stream(upgraded, Scheme::HTTP, authority).await
                                {
                                    log::error!("HTTP connect error, {e}");
                                }
                            }
                            // Tls

                            // Content type: Handshake (22)
                            // TLS version: 1.x (3, _)
                            else if buffer[..2] == [22, 3] {
                                let server_config = self.ca.gen_server_config(&authority).await;

                                log::debug!(
                                    "TLS connections established with client and tunnel server"
                                );

                                // stream for proxy to server
                                let stream =
                                    match TlsAcceptor::from(server_config).accept(upgraded).await {
                                        Ok(stream) => stream,
                                        Err(e) => {
                                            log::error!("Failed to establish TLS connection: {e}");
                                            return;
                                        }
                                    };

                                if let Err(e) =
                                    self.serve_stream(stream, Scheme::HTTPS, authority).await
                                {
                                    if !e.to_string().starts_with("error shutting down connection")
                                    {
                                        log::error!("HTTPS connect error: {e}");
                                    }
                                }
                            } else {
                                log::debug!(
                                    "Unknown protocol, read '{:02X?}' from upgraded connection",
                                    &buffer[..bytes_read]
                                );

                                let mut server = match TcpStream::connect(authority.as_ref()).await
                                {
                                    Ok(server) => server,
                                    Err(e) => {
                                        log::error!("Failed to connect to {authority}: {e}");
                                        return;
                                    }
                                };

                                if let Err(e) =
                                    tokio::io::copy_bidirectional(&mut upgraded, &mut server).await
                                {
                                    log::error!(
                                        "Failed to tunnel unknown protocol to {}: {}",
                                        authority,
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Upgrade error: {e}");
                        }
                    }
                };

                tokio::spawn(fut);
                Response::new(Body::empty())
            }
            None => bad_request(),
        }
    }

    fn send_typed_event(&self, event: CoreEventEnvelope) {
        let Some(transporter) = self.typed_transporter.as_ref() else {
            return;
        };
        let status = core_events::try_send_typed_event(transporter, event);
        if !matches!(status, core_events::TypedEventSendStatus::Sent) {
            log::debug!("drop typed event by typed event backpressure policy: {status:?}");
        }
    }

    fn upgrade_websocket(self, req: Request<Body>) -> Response<Body> {
        let (mut parts, _) = req.into_parts();
        let upstream_uri = match websocket_upstream_uri(&parts.uri) {
            Some(uri) => uri,
            None => return bad_request(),
        };

        parts.uri = match websocket_client_upgrade_uri(&upstream_uri) {
            Some(uri) => uri,
            None => return bad_request(),
        };

        let conn_id = Uuid::new_v4();
        let mut req = Request::from_parts(parts, ());

        match hyper_tungstenite::upgrade(&mut req, None)
            .context(WebsocketProtocolError {})
            .context(ClientError {
                scenario: "upgrade tungstenite",
            }) {
            Ok((res, websocket)) => {
                let fut = async move {
                    match websocket.await {
                        Ok(ws) => {
                            if let Err(e) = self.handle_websocket(conn_id, ws, upstream_uri).await {
                                log::error!("Failed to handle websocket: {e}");
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to upgrade to websocket: {e}");
                        }
                    }
                };

                tokio::spawn(fut);
                res
            }
            Err(_) => bad_request(),
        }
    }

    async fn handle_websocket(
        self,
        conn_id: Uuid,
        client_socket: WebSocketStream<Upgraded>,
        upstream_uri: Uri,
    ) -> Result<(), tungstenite::Error> {
        let (upstream_socket, _) =
            tokio_tungstenite::connect_async(upstream_uri.to_string()).await?;
        let (mut client_write, mut client_read) = client_socket.split();
        let (mut upstream_write, mut upstream_read) = upstream_socket.split();

        loop {
            tokio::select! {
                message = client_read.next() => {
                    let message = match message {
                        Some(message) => message?,
                        None => break,
                    };
                    let should_close = message.is_close();
                    self.emit_websocket_message(
                        conn_id,
                        upstream_uri.clone(),
                        "clientToServer",
                        &message,
                    );
                    upstream_write.send(message).await?;
                    if should_close {
                        break;
                    }
                }
                message = upstream_read.next() => {
                    let message = match message {
                        Some(message) => message?,
                        None => break,
                    };
                    let should_close = message.is_close();
                    self.emit_websocket_message(
                        conn_id,
                        upstream_uri.clone(),
                        "serverToClient",
                        &message,
                    );
                    client_write.send(message).await?;
                    if should_close {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn emit_websocket_message(
        &self,
        conn_id: Uuid,
        uri: Uri,
        direction: &'static str,
        message: &tungstenite::Message,
    ) {
        let preview = websocket_message_preview(message);
        let event = WebSocketMessageEvent::new(
            conn_id,
            uri,
            direction,
            preview.opcode,
            preview.payload,
            preview.payload_len,
            preview.preview_encoding,
            preview.lossy_preview,
            preview.preview_truncated,
            preview.close_code,
            preview.close_reason,
        );

        if let Err(err) = self.transporter.try_send(event.into()) {
            log::debug!("send WebSocket message event to client failed: {err}");
        }
    }

    async fn serve_stream<S>(
        self,
        stream: S,
        scheme: Scheme,
        authority: Authority,
    ) -> Result<(), EndpointError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let service = service_fn(move |mut req| {
            let this = self.clone();
            let scheme = scheme.clone();
            let authority = authority.clone();

            async move {
                if req.version() == hyper::Version::HTTP_10
                    || req.version() == hyper::Version::HTTP_11
                {
                    let (mut parts, body) = req.into_parts();

                    parts.uri = match rewrite_tunneled_request_uri(&parts.uri, scheme, &authority) {
                        Ok(uri) => uri,
                        Err(err) => {
                            log::debug!("failed to rewrite tunneled request URI: {err}");
                            return Ok(bad_request());
                        }
                    };

                    req = Request::from_parts(parts, body);
                };

                this.accept(req).await
            }
        });

        Http::new()
            .serve_connection(stream, service)
            .with_upgrades()
            .await
            .context(HttpError {})
    }
}

fn normalize_request<T>(mut req: Request<T>) -> Request<T> {
    // Hyper will automatically add a Host header if needed.
    req.headers_mut().remove(hyper::header::HOST);

    // HTTP/2 supports multiple cookie headers, but HTTP/1.x only supports one.
    if let Entry::Occupied(mut cookies) = req.headers_mut().entry(hyper::header::COOKIE) {
        let joined_cookies = bstr::join(b"; ", cookies.iter());
        cookies.insert(joined_cookies.try_into().expect("Failed to join cookies"));
    }

    *req.version_mut() = hyper::Version::HTTP_11;

    req
}

fn rewrite_tunneled_request_uri(
    uri: &Uri,
    scheme: Scheme,
    authority: &Authority,
) -> Result<Uri, http::uri::InvalidUriParts> {
    let mut parts = uri.clone().into_parts();
    parts.scheme = Some(scheme);
    parts.authority = Some(authority.clone());
    Uri::from_parts(parts)
}

fn websocket_upstream_uri(uri: &Uri) -> Option<Uri> {
    let mut parts = uri.clone().into_parts();
    let scheme = match parts.scheme.as_ref().map(Scheme::as_str) {
        Some("ws") | Some("http") => "ws",
        Some("wss") | Some("https") => "wss",
        _ => return None,
    };
    parts.scheme = Some(scheme.parse().ok()?);

    Uri::from_parts(parts).ok()
}

fn websocket_client_upgrade_uri(upstream_uri: &Uri) -> Option<Uri> {
    upstream_uri
        .path_and_query()
        .map(|path| path.as_str())
        .unwrap_or("/")
        .parse()
        .ok()
}

struct WebSocketMessagePreview {
    opcode: &'static str,
    payload: String,
    payload_len: usize,
    preview_encoding: &'static str,
    lossy_preview: bool,
    preview_truncated: bool,
    close_code: Option<u16>,
    close_reason: Option<String>,
}

fn websocket_message_preview(message: &tungstenite::Message) -> WebSocketMessagePreview {
    match message {
        tungstenite::Message::Text(payload) => {
            let (payload_preview, preview_encoding, lossy_preview, preview_truncated) =
                websocket_text_payload_preview(payload);
            WebSocketMessagePreview {
                opcode: "text",
                payload: payload_preview,
                payload_len: payload.len(),
                preview_encoding,
                lossy_preview,
                preview_truncated,
                close_code: None,
                close_reason: None,
            }
        }
        tungstenite::Message::Binary(payload) => {
            websocket_binary_message_preview("binary", payload)
        }
        tungstenite::Message::Ping(payload) => websocket_binary_message_preview("ping", payload),
        tungstenite::Message::Pong(payload) => websocket_binary_message_preview("pong", payload),
        tungstenite::Message::Close(Some(frame)) => {
            let reason = frame.reason.to_string();
            let (payload_preview, preview_encoding, lossy_preview, preview_truncated) =
                websocket_text_payload_preview(&reason);
            WebSocketMessagePreview {
                opcode: "close",
                payload: payload_preview,
                payload_len: 2 + reason.as_bytes().len(),
                preview_encoding,
                lossy_preview,
                preview_truncated,
                close_code: Some(u16::from(frame.code)),
                close_reason: Some(reason),
            }
        }
        tungstenite::Message::Close(None) => WebSocketMessagePreview {
            opcode: "close",
            payload: String::new(),
            payload_len: 0,
            preview_encoding: "utf8",
            lossy_preview: false,
            preview_truncated: false,
            close_code: None,
            close_reason: None,
        },
        tungstenite::Message::Frame(_) => WebSocketMessagePreview {
            opcode: "frame",
            payload: String::new(),
            payload_len: 0,
            preview_encoding: "utf8",
            lossy_preview: false,
            preview_truncated: false,
            close_code: None,
            close_reason: None,
        },
    }
}

fn websocket_binary_message_preview(
    opcode: &'static str,
    payload: &[u8],
) -> WebSocketMessagePreview {
    let (payload_preview, preview_encoding, lossy_preview, preview_truncated) =
        websocket_binary_payload_preview(payload);
    WebSocketMessagePreview {
        opcode,
        payload: payload_preview,
        payload_len: payload.len(),
        preview_encoding,
        lossy_preview,
        preview_truncated,
        close_code: None,
        close_reason: None,
    }
}

fn websocket_text_payload_preview(payload: &str) -> (String, &'static str, bool, bool) {
    if payload.len() <= WEBSOCKET_MESSAGE_PREVIEW_LIMIT_BYTES {
        return (payload.to_string(), "utf8", false, false);
    }

    let mut end = 0;
    for (idx, ch) in payload.char_indices() {
        let next = idx + ch.len_utf8();
        if next > WEBSOCKET_MESSAGE_PREVIEW_LIMIT_BYTES {
            break;
        }
        end = next;
    }

    (payload[..end].to_string(), "utf8", false, true)
}

fn websocket_binary_payload_preview(payload: &[u8]) -> (String, &'static str, bool, bool) {
    let preview_len = payload.len().min(WEBSOCKET_MESSAGE_PREVIEW_LIMIT_BYTES);
    let preview = &payload[..preview_len];
    let preview_truncated = payload.len() > WEBSOCKET_MESSAGE_PREVIEW_LIMIT_BYTES;

    match std::str::from_utf8(preview) {
        Ok(preview) => (preview.to_string(), "utf8", false, preview_truncated),
        Err(_) => (
            String::from_utf8_lossy(preview).into_owned(),
            "utf8Lossy",
            true,
            preview_truncated,
        ),
    }
}

fn tap_response_body_for_event(
    conn_id: Uuid,
    req_uri: Uri,
    mut res: Response<Body>,
    effects: Option<crate::processors::processor_effect::ProcessorEffects>,
    transporter: Sender<Events>,
    typed_transporter: Option<Sender<CoreEventEnvelope>>,
) -> Response<Body> {
    let status = res.status();
    let version = res.version();
    let headers = res.headers().clone();
    let sse_parser = is_sse_response(&headers).then(SseParser::default);
    res.headers_mut().remove(CONTENT_LENGTH);
    let body = std::mem::replace(res.body_mut(), Body::empty());

    let stream = futures::stream::unfold(
        (body, Vec::<u8>::new(), sse_parser, 0_usize),
        move |(mut body, mut captured, mut sse_parser, mut offset)| {
            let req_uri = req_uri.clone();
            let headers = headers.clone();
            let effects = effects.clone();
            let transporter = transporter.clone();
            let typed_transporter = typed_transporter.clone();

            async move {
                match body.next().await {
                    Some(Ok(chunk)) => {
                        if let Some(typed_transporter) = typed_transporter.as_ref() {
                            let event = core_events::response_body_chunk_event(
                                conn_id.to_string(),
                                req_uri.to_string(),
                                offset,
                                chunk.as_ref(),
                            );
                            let status =
                                core_events::try_send_typed_event(typed_transporter, event);
                            if !matches!(status, core_events::TypedEventSendStatus::Sent) {
                                log::debug!(
                                    "drop response body chunk event by typed event backpressure policy: {status:?}"
                                );
                            }
                        }

                        if let Some(parser) = sse_parser.as_mut() {
                            for event in parser.push_bytes(&chunk) {
                                let event = ServerSentEvent::new(
                                    conn_id,
                                    req_uri.clone(),
                                    event.event,
                                    event.data,
                                    event.id,
                                    event.retry,
                                );

                                if let Err(err) = transporter.try_send(event.into()) {
                                    log::debug!("send SSE event to client failed: {err}");
                                }
                            }
                        }

                        let remaining = BODY_CAPTURE_LIMIT_BYTES.saturating_sub(captured.len());
                        if remaining > 0 {
                            captured.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                        }
                        offset = offset.saturating_add(chunk.len());
                        Some((
                            Ok::<Bytes, hyper::Error>(chunk),
                            (body, captured, sse_parser, offset),
                        ))
                    }
                    Some(Err(err)) => Some((Err(err), (body, captured, sse_parser, offset))),
                    None => {
                        let event = ResponseEvent::from_body_bytes(
                            conn_id,
                            req_uri,
                            status,
                            version,
                            headers,
                            Bytes::from(captured),
                            effects,
                        );

                        if let Err(err) = transporter.send(event.into()).await {
                            log::error!("send response event to client failed: {err}");
                        }

                        None
                    }
                }
            }
        },
    );

    *res.body_mut() = Body::wrap_stream(stream);
    res
}

fn is_sse_response(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(';')
                .next()
                .map(str::trim)
                .is_some_and(|content_type| content_type.eq_ignore_ascii_case("text/event-stream"))
        })
        .unwrap_or(false)
}

fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::empty())
        .expect("Failed to build response")
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use http::{uri::Authority, uri::Scheme, Uri};
    use tokio_tungstenite::tungstenite;

    #[test]
    fn correctness_tunnel_uri_rewrite_preserves_custom_port() {
        let uri = Uri::from_static("/resource?x=1");
        let authority = Authority::from_str("example.test:8443").expect("authority");

        let rewritten = super::rewrite_tunneled_request_uri(&uri, Scheme::HTTPS, &authority)
            .expect("rewrite should succeed");

        assert_eq!(
            rewritten.to_string(),
            "https://example.test:8443/resource?x=1"
        );
    }

    #[test]
    fn websocket_preview_marks_lossy_binary_payload() {
        let preview =
            super::websocket_message_preview(&tungstenite::Message::Binary(vec![0xff, 0xfe, b'a']));

        assert_eq!(preview.opcode, "binary");
        assert_eq!(preview.payload_len, 3);
        assert_eq!(preview.preview_encoding, "utf8Lossy");
        assert!(preview.lossy_preview);
        assert!(!preview.preview_truncated);
    }

    #[test]
    fn websocket_preview_truncates_large_payload() {
        let preview = super::websocket_message_preview(&tungstenite::Message::Binary(vec![
            b'x';
            super::WEBSOCKET_MESSAGE_PREVIEW_LIMIT_BYTES + 1
        ]));

        assert_eq!(
            preview.payload_len,
            super::WEBSOCKET_MESSAGE_PREVIEW_LIMIT_BYTES + 1
        );
        assert_eq!(
            preview.payload.len(),
            super::WEBSOCKET_MESSAGE_PREVIEW_LIMIT_BYTES
        );
        assert_eq!(preview.preview_encoding, "utf8");
        assert!(!preview.lossy_preview);
        assert!(preview.preview_truncated);
    }
}
