use std::{convert::Infallible, future::Future, net::SocketAddr, sync::Arc};
use tauri::async_runtime::Mutex;
use tokio::sync::mpsc::Sender;

use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Client, Server,
};
use hyper_rustls::HttpsConnectorBuilder;
use snafu::ResultExt;

use super::tunnel::Tunnel;

use crate::{
    ca::Ssl,
    core_api::events::CoreEventEnvelope,
    error::{
        self,
        endpoint_error::{ConnectError, HttpError},
        ServerError,
    },
    events,
    processors::http_processor::HttpProcessor,
};

pub struct ProxyService {
    addr: SocketAddr,
    transporter: Option<Sender<events::Events>>,
    typed_transporter: Option<Sender<CoreEventEnvelope>>,
    processor: Arc<Mutex<HttpProcessor>>,
    ca: Arc<Ssl>,
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, sync::Arc};

    use bytes::Bytes;
    use futures::{SinkExt, StreamExt};
    use hyper::{
        header::CONTENT_TYPE,
        service::{make_service_fn, service_fn},
        Body, Response, Server,
    };
    use tauri::async_runtime::Mutex;
    use tokio::sync::{mpsc, oneshot};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        time::{sleep, timeout, Duration},
    };
    use tokio_tungstenite::{accept_async, tungstenite};

    use super::ProxyService;
    use crate::{
        ca::Ssl,
        core_api::events::{
            legacy_event_to_core_events, BodyPreviewEncoding, CoreEvent,
            WebSocketMessageEvent as CoreWebSocketMessageEvent,
        },
        events::Events,
        processors::{
            http_processor::{
                delay::RequestDelayProcessor, request_header::RequestHeaderProcessor,
                response_header::ResponseHeaderProcessor, typed_rules::TypedRuleProcessor,
                HttpProcessor,
            },
            processor_pack::ProcessorPack,
        },
    };

    fn reserve_local_addr() -> SocketAddr {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("failed to reserve local port");
        let addr = listener.local_addr().expect("failed to read local addr");
        drop(listener);
        addr
    }

    async fn spawn_origin() -> (SocketAddr, oneshot::Sender<()>) {
        let addr = reserve_local_addr();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let make_service = make_service_fn(|_| async {
            Ok::<_, hyper::Error>(service_fn(|_req| async {
                Ok::<_, hyper::Error>(Response::new(Body::from("origin-ok")))
            }))
        });

        tokio::spawn(async move {
            let _ = Server::bind(&addr)
                .serve(make_service)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        (addr, shutdown_tx)
    }

    async fn spawn_streaming_origin() -> (SocketAddr, oneshot::Sender<()>) {
        let addr = reserve_local_addr();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let make_service = make_service_fn(|_| async {
            Ok::<_, hyper::Error>(service_fn(|_req| async {
                let (mut sender, body) = Body::channel();

                tokio::spawn(async move {
                    let _ = sender.send_data(Bytes::from_static(b"first-")).await;
                    sleep(Duration::from_millis(1_000)).await;
                    let _ = sender.send_data(Bytes::from_static(b"second")).await;
                });

                Ok::<_, hyper::Error>(Response::new(body))
            }))
        });

        tokio::spawn(async move {
            let _ = Server::bind(&addr)
                .serve(make_service)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        (addr, shutdown_tx)
    }

    async fn spawn_sse_origin() -> (SocketAddr, oneshot::Sender<()>) {
        let addr = reserve_local_addr();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let make_service = make_service_fn(|_| async {
            Ok::<_, hyper::Error>(service_fn(|_req| async {
                let (mut sender, body) = Body::channel();

                tokio::spawn(async move {
                    let _ = sender
                        .send_data(Bytes::from_static(
                            b"event: greeting\ndata: hello\nid: 7\n\n",
                        ))
                        .await;
                    sleep(Duration::from_millis(1_000)).await;
                    let _ = sender
                        .send_data(Bytes::from_static(b"event: done\ndata: bye\n\n"))
                        .await;
                });

                Ok::<_, hyper::Error>(
                    Response::builder()
                        .header(CONTENT_TYPE, "text/event-stream")
                        .body(body)
                        .expect("failed to build sse response"),
                )
            }))
        });

        tokio::spawn(async move {
            let _ = Server::bind(&addr)
                .serve(make_service)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        (addr, shutdown_tx)
    }

    async fn spawn_fixed_body_origin(body: Vec<u8>) -> (SocketAddr, oneshot::Sender<()>) {
        let addr = reserve_local_addr();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let body = Arc::new(body);
        let make_service = make_service_fn(move |_| {
            let body = Arc::clone(&body);

            async move {
                Ok::<_, hyper::Error>(service_fn(move |_req| {
                    let body = Arc::clone(&body);

                    async move { Ok::<_, hyper::Error>(Response::new(Body::from((*body).clone()))) }
                }))
            }
        });

        tokio::spawn(async move {
            let _ = Server::bind(&addr)
                .serve(make_service)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        (addr, shutdown_tx)
    }

    async fn spawn_websocket_echo_origin() -> (SocketAddr, oneshot::Sender<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind websocket echo origin");
        let addr = listener
            .local_addr()
            .expect("failed to read websocket addr");
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => {
                        let (stream, _) = match accepted {
                            Ok(accepted) => accepted,
                            Err(_) => break,
                        };

                        tokio::spawn(async move {
                            let mut websocket = match accept_async(stream).await {
                                Ok(websocket) => websocket,
                                Err(_) => return,
                            };

                            while let Some(message) = websocket.next().await {
                                let message = match message {
                                    Ok(message) => message,
                                    Err(_) => break,
                                };

                                if websocket.send(message).await.is_err() {
                                    break;
                                }
                            }
                        });
                    }
                }
            }
        });

        (addr, shutdown_tx)
    }

    async fn spawn_websocket_recording_origin() -> (
        SocketAddr,
        oneshot::Sender<()>,
        mpsc::Receiver<tungstenite::Message>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind websocket recording origin");
        let addr = listener
            .local_addr()
            .expect("failed to read websocket addr");
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let (record_tx, record_rx) = mpsc::channel::<tungstenite::Message>(8);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => {
                        let (stream, _) = match accepted {
                            Ok(accepted) => accepted,
                            Err(_) => break,
                        };
                        let record_tx = record_tx.clone();

                        tokio::spawn(async move {
                            let mut websocket = match accept_async(stream).await {
                                Ok(websocket) => websocket,
                                Err(_) => return,
                            };

                            while let Some(message) = websocket.next().await {
                                let message = match message {
                                    Ok(message) => message,
                                    Err(_) => break,
                                };

                                if record_tx.send(message.clone()).await.is_err() {
                                    break;
                                }

                                match message {
                                    tungstenite::Message::Ping(payload) => {
                                        if websocket
                                            .send(tungstenite::Message::Pong(payload))
                                            .await
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                    tungstenite::Message::Close(frame) => {
                                        let _ = websocket
                                            .send(tungstenite::Message::Close(frame))
                                            .await;
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        });
                    }
                }
            }
        });

        (addr, shutdown_tx, record_rx)
    }

    async fn connect_with_retry(addr: SocketAddr) -> TcpStream {
        let mut last_err = None;

        for _ in 0..50 {
            match TcpStream::connect(addr).await {
                Ok(stream) => return stream,
                Err(err) => {
                    last_err = Some(err);
                    sleep(Duration::from_millis(10)).await;
                }
            }
        }

        panic!(
            "failed to connect to {addr}: {}",
            last_err
                .map(|err| err.to_string())
                .unwrap_or_else(|| "unknown error".to_string())
        );
    }

    async fn read_http_response_head(stream: &mut TcpStream) -> String {
        let mut response = Vec::new();
        let mut buf = [0_u8; 1];

        loop {
            stream
                .read_exact(&mut buf)
                .await
                .expect("failed to read websocket handshake response");
            response.push(buf[0]);

            if response.ends_with(b"\r\n\r\n") {
                break;
            }
        }

        String::from_utf8(response).expect("handshake response was not utf8")
    }

    async fn write_masked_text_frame(stream: &mut TcpStream, text: &str) {
        write_masked_frame(stream, 0x1, text.as_bytes()).await;
    }

    async fn write_masked_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) {
        assert!(payload.len() < 126);

        let mask = [1_u8, 2, 3, 4];
        let mut frame = vec![0x80 | opcode, 0x80 | payload.len() as u8];
        frame.extend_from_slice(&mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(idx, byte)| byte ^ mask[idx % 4]),
        );

        stream
            .write_all(&frame)
            .await
            .expect("failed to write websocket frame");
    }

    async fn read_unmasked_text_frame(stream: &mut TcpStream) -> String {
        let (opcode, payload) = read_unmasked_frame(stream).await;
        assert_eq!(opcode, 1, "expected text websocket frame");

        String::from_utf8(payload).expect("websocket text frame was not utf8")
    }

    async fn read_unmasked_frame(stream: &mut TcpStream) -> (u8, Vec<u8>) {
        let mut header = [0_u8; 2];
        stream
            .read_exact(&mut header)
            .await
            .expect("failed to read websocket frame header");

        assert_eq!(header[1] & 0x80, 0, "server frames must not be masked");

        let len = (header[1] & 0x7f) as usize;
        assert!(len < 126);

        let mut payload = vec![0_u8; len];
        stream
            .read_exact(&mut payload)
            .await
            .expect("failed to read websocket frame payload");

        (header[0] & 0x0f, payload)
    }

    async fn next_websocket_core_event(
        event_rx: &mut mpsc::Receiver<Events>,
    ) -> CoreWebSocketMessageEvent {
        next_matching_websocket_core_event(event_rx, |_| true).await
    }

    async fn next_matching_websocket_core_event<F>(
        event_rx: &mut mpsc::Receiver<Events>,
        mut matches: F,
    ) -> CoreWebSocketMessageEvent
    where
        F: FnMut(&CoreWebSocketMessageEvent) -> bool,
    {
        loop {
            let event = timeout(Duration::from_secs(1), event_rx.recv())
                .await
                .expect("timed out waiting for websocket message event")
                .expect("event channel closed");

            for core_event in
                legacy_event_to_core_events(&event).expect("failed to map websocket event")
            {
                if let CoreEvent::WebSocketMessage(message) = core_event.event {
                    if matches(&message) {
                        return message;
                    }
                }
            }
        }
    }

    async fn read_full_http_response(stream: &mut TcpStream) -> String {
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .expect("failed to read HTTP response");

        String::from_utf8_lossy(&response).into_owned()
    }

    #[tokio::test]
    async fn proxy_core_forwards_http_request_without_tauri() {
        let (origin_addr, origin_shutdown) = spawn_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, mut event_rx) = mpsc::channel::<Events>(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET http://{origin_addr}/through-proxy HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write proxied request");

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .expect("failed to read proxied response");

        let response = String::from_utf8_lossy(&response);
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(response.contains("origin-ok"), "{response}");
        assert!(matches!(event_rx.recv().await, Some(Events::NewRequest(_))));
        assert!(matches!(
            event_rx.recv().await,
            Some(Events::NewResponse(_))
        ));

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn correctness_unsupported_request_encoding_returns_bad_request_without_crashing() {
        let (origin_addr, origin_shutdown) = spawn_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "POST http://{origin_addr}/unsupported-encoding HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Content-Encoding: snappy\r\n\
             Content-Length: 4\r\n\
             Connection: close\r\n\
             \r\n\
             body"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write proxied request");

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .expect("failed to read proxied response");

        let response = String::from_utf8_lossy(&response);
        assert!(
            response.starts_with("HTTP/1.1 400 Bad Request"),
            "{response}"
        );

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn correctness_unsupported_request_encoding_emits_typed_exchange_error() {
        let (origin_addr, origin_shutdown) = spawn_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(8);
        let (typed_event_tx, mut typed_event_rx) = mpsc::channel(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::with_ca_and_typed_events(
            proxy_addr,
            Some(event_tx),
            Some(typed_event_tx),
            processor,
            Arc::new(Ssl::default()),
        );
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "POST http://{origin_addr}/unsupported-encoding HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Content-Encoding: snappy\r\n\
             Content-Length: 4\r\n\
             Connection: close\r\n\
             \r\n\
             body"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write proxied request");

        let typed_event = timeout(Duration::from_secs(1), typed_event_rx.recv())
            .await
            .expect("timed out waiting for exchange error")
            .expect("typed event channel closed");

        match typed_event.event {
            CoreEvent::ExchangeError(error) => {
                assert_eq!(error.exchange_id.len(), 36);
                assert_eq!(
                    error.uri.as_deref(),
                    Some(format!("http://{origin_addr}/unsupported-encoding").as_str())
                );
                assert_eq!(error.phase, crate::core_api::events::ExchangeErrorPhase::RequestDecode);
                assert!(!error.recoverable);
            }
            other => panic!("unexpected typed event: {other:?}"),
        }

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn streaming_forwards_response_chunk_before_response_eof() {
        let (origin_addr, origin_shutdown) = spawn_streaming_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET http://{origin_addr}/streaming HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write proxied request");

        let mut collected = Vec::new();
        let mut buf = [0_u8; 256];
        let first_chunk = timeout(Duration::from_millis(300), async {
            loop {
                let n = stream
                    .read(&mut buf)
                    .await
                    .expect("failed to read proxied stream");
                assert!(n > 0, "stream closed before first chunk");
                collected.extend_from_slice(&buf[..n]);

                if collected
                    .windows(b"first-".len())
                    .any(|part| part == b"first-")
                {
                    break;
                }
            }
        })
        .await;

        assert!(
            first_chunk.is_ok(),
            "first response chunk was not forwarded before EOF"
        );

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn streaming_emits_typed_response_body_chunk_event() {
        let (origin_addr, origin_shutdown) = spawn_streaming_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(8);
        let (typed_event_tx, mut typed_event_rx) = mpsc::channel(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::with_ca_and_typed_events(
            proxy_addr,
            Some(event_tx),
            Some(typed_event_tx),
            processor,
            Arc::new(Ssl::default()),
        );
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET http://{origin_addr}/streaming HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write proxied request");

        let chunk_event = timeout(Duration::from_secs(1), typed_event_rx.recv())
            .await
            .expect("timed out waiting for typed body chunk")
            .expect("typed event channel closed");

        match chunk_event.event {
            CoreEvent::ResponseBodyChunk(chunk) => {
                assert_eq!(chunk.exchange_id.len(), 36);
                assert_eq!(chunk.uri, format!("http://{origin_addr}/streaming"));
                assert_eq!(chunk.offset, 0);
                assert_eq!(chunk.byte_len, "first-".len());
                assert_eq!(chunk.preview, "first-");
                assert!(!chunk.preview_truncated);
            }
            other => panic!("unexpected typed event: {other:?}"),
        }

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn streaming_limits_captured_response_body_but_forwards_full_body() {
        const EXPECTED_CAPTURE_LIMIT: usize = 64 * 1024;

        let full_body = vec![b'X'; EXPECTED_CAPTURE_LIMIT + 1024];
        let (origin_addr, origin_shutdown) = spawn_fixed_body_origin(full_body.clone()).await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, mut event_rx) = mpsc::channel::<Events>(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET http://{origin_addr}/large HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write proxied request");

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .expect("failed to read proxied response");

        let forwarded_body_bytes = response.iter().filter(|byte| **byte == b'X').count();
        assert_eq!(
            forwarded_body_bytes,
            full_body.len(),
            "full response body was not forwarded"
        );

        assert!(matches!(event_rx.recv().await, Some(Events::NewRequest(_))));
        let event = timeout(Duration::from_secs(1), event_rx.recv())
            .await
            .expect("timed out waiting for response event")
            .expect("event channel closed");
        let json = serde_json::to_value(event).expect("failed to serialize event");
        let captured_body = json
            .get("NewResponse")
            .and_then(|event| event.get("body"))
            .and_then(|body| body.as_str())
            .expect("missing response body in event");

        assert_eq!(captured_body.len(), EXPECTED_CAPTURE_LIMIT);

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn sse_proxy_emits_event_before_stream_eof() {
        let (origin_addr, origin_shutdown) = spawn_sse_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, mut event_rx) = mpsc::channel::<Events>(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET http://{origin_addr}/events HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write proxied request");

        let mut collected = Vec::new();
        let mut buf = [0_u8; 256];
        timeout(Duration::from_millis(300), async {
            loop {
                let n = stream
                    .read(&mut buf)
                    .await
                    .expect("failed to read proxied sse stream");
                assert!(n > 0, "stream closed before first SSE event");
                collected.extend_from_slice(&buf[..n]);

                if collected
                    .windows(b"hello".len())
                    .any(|part| part == b"hello")
                {
                    break;
                }
            }
        })
        .await
        .expect("first SSE payload was not forwarded before EOF");

        assert!(matches!(event_rx.recv().await, Some(Events::NewRequest(_))));
        let event = timeout(Duration::from_millis(300), event_rx.recv())
            .await
            .expect("timed out waiting for SSE event")
            .expect("event channel closed");

        assert!(matches!(event, Events::SseEvent(_)));
        let json = serde_json::to_value(event).expect("failed to serialize event");
        let event = json.get("SseEvent").expect("missing SseEvent payload");
        assert_eq!(
            event.get("event").and_then(|value| value.as_str()),
            Some("greeting")
        );
        assert_eq!(
            event.get("data").and_then(|value| value.as_str()),
            Some("hello")
        );
        assert_eq!(
            event.get("lastEventId").and_then(|value| value.as_str()),
            Some("7")
        );

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn websocket_forwards_text_messages_bidirectionally() {
        let (origin_addr, origin_shutdown) = spawn_websocket_echo_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, mut event_rx) = mpsc::channel::<Events>(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET ws://{origin_addr}/echo HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: Upgrade\r\n\
             Upgrade: websocket\r\n\
             Sec-WebSocket-Version: 13\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write websocket handshake");

        let response = read_http_response_head(&mut stream).await;
        assert!(response.starts_with("HTTP/1.1 101"), "{response}");

        write_masked_text_frame(&mut stream, "through-proxy").await;
        let message = timeout(
            Duration::from_secs(1),
            read_unmasked_text_frame(&mut stream),
        )
        .await
        .expect("timed out waiting for websocket echo");

        assert_eq!(message, "through-proxy");

        let first_event = timeout(Duration::from_secs(1), event_rx.recv())
            .await
            .expect("timed out waiting for websocket message event")
            .expect("event channel closed");
        let second_event = timeout(Duration::from_secs(1), event_rx.recv())
            .await
            .expect("timed out waiting for websocket message event")
            .expect("event channel closed");

        assert!(matches!(first_event, Events::WebSocketMessage(_)));
        assert!(matches!(second_event, Events::WebSocketMessage(_)));

        let first = serde_json::to_value(first_event).expect("failed to serialize event");
        let second = serde_json::to_value(second_event).expect("failed to serialize event");
        assert_eq!(
            first
                .get("WebSocketMessage")
                .and_then(|event| event.get("direction"))
                .and_then(|direction| direction.as_str()),
            Some("clientToServer")
        );
        assert_eq!(
            second
                .get("WebSocketMessage")
                .and_then(|event| event.get("direction"))
                .and_then(|direction| direction.as_str()),
            Some("serverToClient")
        );

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn websocket_forwards_binary_messages_and_records_typed_metadata() {
        let (origin_addr, origin_shutdown) = spawn_websocket_echo_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, mut event_rx) = mpsc::channel::<Events>(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET ws://{origin_addr}/echo HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: Upgrade\r\n\
             Upgrade: websocket\r\n\
             Sec-WebSocket-Version: 13\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write websocket handshake");

        let response = read_http_response_head(&mut stream).await;
        assert!(response.starts_with("HTTP/1.1 101"), "{response}");

        write_masked_frame(&mut stream, 0x2, &[0xff, 0xfe, b'a']).await;
        let (opcode, payload) = timeout(Duration::from_secs(1), read_unmasked_frame(&mut stream))
            .await
            .expect("timed out waiting for websocket binary echo");

        assert_eq!(opcode, 0x2);
        assert_eq!(payload, vec![0xff, 0xfe, b'a']);

        let first = next_websocket_core_event(&mut event_rx).await;
        let second = next_websocket_core_event(&mut event_rx).await;

        assert_eq!(first.direction, "clientToServer");
        assert_eq!(first.opcode, "binary");
        assert_eq!(first.payload_len, 3);
        assert_eq!(first.preview_encoding, BodyPreviewEncoding::Utf8Lossy);
        assert!(first.lossy_preview);
        assert!(!first.preview_truncated);
        assert_eq!(first.close_code, None);
        assert_eq!(first.close_reason, None);

        assert_eq!(second.direction, "serverToClient");
        assert_eq!(second.opcode, "binary");
        assert_eq!(second.payload_len, 3);
        assert_eq!(second.preview_encoding, BodyPreviewEncoding::Utf8Lossy);
        assert!(second.lossy_preview);
        assert!(!second.preview_truncated);

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn websocket_forwards_control_frames_and_records_typed_metadata() {
        let (origin_addr, origin_shutdown, mut origin_rx) = spawn_websocket_recording_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, mut event_rx) = mpsc::channel::<Events>(8);
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET ws://{origin_addr}/control HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: Upgrade\r\n\
             Upgrade: websocket\r\n\
             Sec-WebSocket-Version: 13\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write websocket handshake");

        let response = read_http_response_head(&mut stream).await;
        assert!(response.starts_with("HTTP/1.1 101"), "{response}");

        write_masked_frame(&mut stream, 0x9, b"hi").await;
        let origin_ping = timeout(Duration::from_secs(1), origin_rx.recv())
            .await
            .expect("origin did not receive ping")
            .expect("origin event channel closed");
        assert!(matches!(origin_ping, tungstenite::Message::Ping(payload) if payload == b"hi"));
        let (opcode, payload) = timeout(Duration::from_secs(1), read_unmasked_frame(&mut stream))
            .await
            .expect("timed out waiting for websocket pong");
        assert_eq!(opcode, 0xA);
        assert_eq!(payload, b"hi");

        let ping = next_matching_websocket_core_event(&mut event_rx, |message| {
            message.direction == "clientToServer" && message.opcode == "ping"
        })
        .await;
        assert_eq!(ping.payload_preview, "hi");
        assert_eq!(ping.payload_len, 2);
        assert_eq!(ping.preview_encoding, BodyPreviewEncoding::Utf8);
        assert!(!ping.lossy_preview);

        write_masked_frame(&mut stream, 0xA, b"ok").await;
        let origin_pong = timeout(Duration::from_secs(1), origin_rx.recv())
            .await
            .expect("origin did not receive pong")
            .expect("origin event channel closed");
        assert!(
            matches!(origin_pong, tungstenite::Message::Pong(payload) if payload == b"ok")
        );

        let pong = next_matching_websocket_core_event(&mut event_rx, |message| {
            message.direction == "clientToServer" && message.opcode == "pong"
        })
        .await;
        assert_eq!(pong.payload_preview, "ok");
        assert_eq!(pong.payload_len, 2);
        assert_eq!(pong.preview_encoding, BodyPreviewEncoding::Utf8);
        assert!(!pong.lossy_preview);

        let mut close_payload = 1000_u16.to_be_bytes().to_vec();
        close_payload.extend_from_slice(b"done");
        write_masked_frame(&mut stream, 0x8, &close_payload).await;
        let origin_close = timeout(Duration::from_secs(1), origin_rx.recv())
            .await
            .expect("origin did not receive close")
            .expect("origin event channel closed");
        assert!(
            matches!(
                origin_close,
                tungstenite::Message::Close(Some(frame))
                    if u16::from(frame.code) == 1000 && frame.reason == "done"
            )
        );

        let close = next_matching_websocket_core_event(&mut event_rx, |message| {
            message.direction == "clientToServer" && message.opcode == "close"
        })
        .await;
        assert_eq!(close.payload_preview, "done");
        assert_eq!(close.payload_len, 6);
        assert_eq!(close.preview_encoding, BodyPreviewEncoding::Utf8);
        assert!(!close.lossy_preview);
        assert_eq!(close.close_code, Some(1000));
        assert_eq!(close.close_reason.as_deref(), Some("done"));

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn rules_delay_does_not_block_unmatched_concurrent_request() {
        let (origin_addr, origin_shutdown) = spawn_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(16);
        let mut pack = ProcessorPack::new("test".to_string(), true);
        pack.set_delay(RequestDelayProcessor::from(format!(
            "http://{origin_addr}/slow 500"
        )));
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![pack])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut slow_stream = connect_with_retry(proxy_addr).await;
        let slow_request = format!(
            "GET http://{origin_addr}/slow HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        slow_stream
            .write_all(slow_request.as_bytes())
            .await
            .expect("failed to write slow request");
        let slow_task =
            tokio::spawn(async move { read_full_http_response(&mut slow_stream).await });

        sleep(Duration::from_millis(50)).await;

        let mut fast_stream = connect_with_retry(proxy_addr).await;
        let fast_request = format!(
            "GET http://{origin_addr}/fast HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        fast_stream
            .write_all(fast_request.as_bytes())
            .await
            .expect("failed to write fast request");

        let fast_response = timeout(
            Duration::from_millis(250),
            read_full_http_response(&mut fast_stream),
        )
        .await
        .expect("fast request was blocked by delayed request");
        assert!(fast_response.contains("origin-ok"), "{fast_response}");

        let _ = slow_task.await;
        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn rules_response_header_processor_mutates_upstream_response() {
        let (origin_addr, origin_shutdown) = spawn_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(16);
        let mut pack = ProcessorPack::new("test".to_string(), true);
        pack.set_response_header(ResponseHeaderProcessor::from(
            "200 x-proxyman-rule matched".to_string(),
        ));
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![pack])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET http://{origin_addr}/header HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write request");
        let response = read_full_http_response(&mut stream).await;

        assert!(
            response.to_lowercase().contains("x-proxyman-rule: matched"),
            "{response}"
        );

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn rules_request_header_processor_mutates_upstream_request() {
        let origin_addr = reserve_local_addr();
        let (origin_shutdown_tx, origin_shutdown_rx) = oneshot::channel::<()>();
        let make_service = make_service_fn(|_| async {
            Ok::<_, hyper::Error>(service_fn(|req: hyper::Request<Body>| async move {
                let header = req
                    .headers()
                    .get("x-proxyman-request")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("missing")
                    .to_string();

                Ok::<_, hyper::Error>(Response::new(Body::from(header)))
            }))
        });
        let origin_server = Server::bind(&origin_addr)
            .serve(make_service)
            .with_graceful_shutdown(async {
                let _ = origin_shutdown_rx.await;
            });
        tokio::spawn(origin_server);

        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(16);
        let mut pack = ProcessorPack::new("test".to_string(), true);
        pack.set_request_header(RequestHeaderProcessor::from(
            "GET .* x-proxyman-request matched".to_string(),
        ));
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![pack])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET http://{origin_addr}/request-header HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write request");
        let response = read_full_http_response(&mut stream).await;

        assert!(response.contains("matched"), "{response}");

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown_tx.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn rules_typed_block_returns_response_without_upstream() {
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(16);
        let mut pack = ProcessorPack::new("test".to_string(), true);
        pack.set_typed_rules(
            TypedRuleProcessor::from_rules(
                crate::core_api::rules::parse_typed_rules_dsl(
                    "block * http://blocked.example.test/* 451 blocked-by-rule",
                )
                .expect("rules should parse"),
            )
            .expect("rules should compile"),
        );
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![pack])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = "GET http://blocked.example.test/ad HTTP/1.1\r\n\
             Host: blocked.example.test\r\n\
             Connection: close\r\n\
             \r\n";
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write request");
        let response = read_full_http_response(&mut stream).await;

        assert!(response.starts_with("HTTP/1.1 451"), "{response}");
        assert!(response.contains("blocked-by-rule"), "{response}");

        let _ = proxy_shutdown_tx.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }

    #[tokio::test]
    async fn rules_typed_response_header_mutates_matching_response() {
        let (origin_addr, origin_shutdown) = spawn_origin().await;
        let proxy_addr = reserve_local_addr();
        let (proxy_shutdown_tx, proxy_shutdown_rx) = oneshot::channel::<()>();
        let (event_tx, _event_rx) = mpsc::channel::<Events>(16);
        let mut pack = ProcessorPack::new("test".to_string(), true);
        pack.set_typed_rules(
            TypedRuleProcessor::from_rules(
                crate::core_api::rules::parse_typed_rules_dsl(format!(
                    "response-header 200 http://{origin_addr}/* x-typed-response matched"
                )
                .as_str())
                .expect("rules should parse"),
            )
            .expect("rules should compile"),
        );
        let processor = Arc::new(Mutex::new(HttpProcessor::new(vec![pack])));

        let proxy = ProxyService::new(proxy_addr, Some(event_tx), processor);
        let proxy_task = tokio::spawn(async move {
            proxy
                .start(async {
                    let _ = proxy_shutdown_rx.await;
                })
                .await
        });

        let mut stream = connect_with_retry(proxy_addr).await;
        let request = format!(
            "GET http://{origin_addr}/header HTTP/1.1\r\n\
             Host: {origin_addr}\r\n\
             Connection: close\r\n\
             \r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write request");
        let response = read_full_http_response(&mut stream).await;

        assert!(
            response
                .to_lowercase()
                .contains("x-typed-response: matched"),
            "{response}"
        );

        let _ = proxy_shutdown_tx.send(());
        let _ = origin_shutdown.send(());
        proxy_task
            .await
            .expect("proxy task panicked")
            .expect("proxy failed");
    }
}

impl ProxyService {
    pub fn new(
        addr: SocketAddr,
        transporter: Option<Sender<events::Events>>,
        processor: Arc<Mutex<HttpProcessor>>,
    ) -> Self {
        Self::with_ca(addr, transporter, processor, Arc::new(Ssl::default()))
    }

    pub fn with_ca(
        addr: SocketAddr,
        transporter: Option<Sender<events::Events>>,
        processor: Arc<Mutex<HttpProcessor>>,
        ca: Arc<Ssl>,
    ) -> Self {
        Self::with_ca_and_typed_events(addr, transporter, None, processor, ca)
    }

    pub fn with_ca_and_typed_events(
        addr: SocketAddr,
        transporter: Option<Sender<events::Events>>,
        typed_transporter: Option<Sender<CoreEventEnvelope>>,
        processor: Arc<Mutex<HttpProcessor>>,
        ca: Arc<Ssl>,
    ) -> Self {
        Self {
            addr,
            transporter,
            typed_transporter,
            processor,
            ca,
        }
    }

    pub async fn start<F: Future<Output = ()>>(
        self,
        should_shutdown_signal: F,
    ) -> Result<(), error::Error> {
        let addr = self.addr;

        let connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1();

        #[cfg(feature = "http2")]
        let connector = connector.enable_http2();

        let connector = connector.build();

        let client = Client::builder()
            .http1_preserve_header_case(true)
            .http1_title_case_headers(true)
            .build(connector);

        let server_builder = Server::try_bind(&addr)
            .context(ConnectError {})
            .context(ServerError {
                scenario: "Port was occupied",
            })?
            .http1_preserve_header_case(true)
            .http1_title_case_headers(true);

        let ssl = Arc::clone(&self.ca);

        let make_service = make_service_fn(move |_conn: &AddrStream| {
            let client = client.clone();
            let ca = Arc::clone(&ssl);
            let transporter = self.transporter.clone();
            let typed_transporter = self.typed_transporter.clone();
            let processor = Arc::clone(&self.processor);
            let websocket_connector = None;

            // accept every request with async tasks
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    Tunnel {
                        ca: Arc::clone(&ca),
                        client: client.clone(),
                        websocket_connector: websocket_connector.clone(),
                        transporter: transporter.clone().unwrap(),
                        typed_transporter: typed_transporter.clone(),
                        processor: Arc::clone(&processor),
                    }
                    .accept(req)
                }))
            }
        });

        server_builder
            .serve(make_service)
            .with_graceful_shutdown(should_shutdown_signal)
            .await
            .context(HttpError {})
            .context(ServerError {
                scenario: "tunnel start",
            })
    }
}
