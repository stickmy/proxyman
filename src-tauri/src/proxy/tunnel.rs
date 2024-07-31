use std::{convert::Infallible, sync::Arc};

use bytes::Bytes;
use http::{
    uri::{Authority, Scheme},
    Method, StatusCode, Uri,
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
    error::{
        endpoint_error::{EndpointError, HttpError, WebsocketProtocolError},
        ClientError, ServerError,
    },
    events::{Events, RequestEvent, ResponseEvent},
};

use super::decoder::{decode_request, decode_response};
use super::rewind::Rewind;
use crate::processors::processor;

pub struct Tunnel<CA, C, P> {
    pub ca: Arc<CA>,
    pub client: Client<C>,
    pub websocket_connector: Option<Connector>,
    pub transporter: Sender<Events>,
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

            let mut req = decode_request(req)
                .context(ClientError {
                    scenario: "decoding request body failed",
                })
                .unwrap();

            self.send_event(RequestEvent::new(conn_id, &mut req).await.into())
                .await;

            let processor = self.processor.lock().await;
            let req_or_res = processor.process_request(req).await;
            drop(processor); // release mutex lock

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

            let processor = self.processor.lock().await;
            let mut res = match res {
                Ok(res) => res,
                Err(e) => processor.process_error(e).await,
            };
            drop(processor);

            res = decode_response(res).unwrap();

            self.send_event(
                ResponseEvent::new(conn_id, req_uri, &mut res, req_or_res.processor_effects)
                    .await
                    .into(),
            )
            .await;

            Ok(res)
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

    fn upgrade_websocket(self, req: Request<Body>) -> Response<Body> {
        let mut req = {
            let (mut parts, _) = req.into_parts();

            parts.uri = {
                let mut parts = parts.uri.into_parts();

                parts.scheme = if parts.scheme.unwrap_or(Scheme::HTTP) == Scheme::HTTP {
                    Some("ws".try_into().unwrap())
                } else {
                    Some("wss".try_into().unwrap())
                };

                match Uri::from_parts(parts) {
                    Ok(uri) => uri,
                    Err(_) => return bad_request(),
                }
            };

            Request::from_parts(parts, ())
        };

        match hyper_tungstenite::upgrade(&mut req, None)
            .context(WebsocketProtocolError {})
            .context(ClientError {
                scenario: "upgrade tungstenite",
            }) {
            Ok((res, websocket)) => {
                let fut = async move {
                    match websocket.await {
                        Ok(ws) => {
                            if let Err(e) = self.handle_websocket(ws, req).await {
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
        _server_socket: WebSocketStream<Upgraded>,
        _req: Request<()>,
    ) -> Result<(), tungstenite::Error> {
        Ok(())
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
        let service = service_fn(|mut req| {
            if req.version() == hyper::Version::HTTP_10 || req.version() == hyper::Version::HTTP_11
            {
                let (mut parts, body) = req.into_parts();

                // Use host part only.(Remove port 443)
                let host = authority.host();

                parts.uri = {
                    let mut parts = parts.uri.into_parts();
                    parts.scheme = Some(scheme.clone());
                    parts.authority =
                        Some(Authority::try_from(host).expect("Failed to parse authority"));
                    Uri::from_parts(parts).expect("Failed to build URI")
                };

                req = Request::from_parts(parts, body);
            };

            self.clone().accept(req)
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

fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::empty())
        .expect("Failed to build response")
}
