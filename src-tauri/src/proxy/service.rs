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
    processor: Arc<Mutex<HttpProcessor>>,
}

impl ProxyService {
    pub fn new(
        addr: SocketAddr,
        transporter: Option<Sender<events::Events>>,
        processor: Arc<Mutex<HttpProcessor>>,
    ) -> Self {
        Self {
            addr,
            transporter,
            processor,
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

        let ssl = Arc::new(Ssl::default());

        let make_service = make_service_fn(move |conn: &AddrStream| {
            let client = client.clone();
            let ca = Arc::clone(&ssl);
            let transporter = self.transporter.clone();
            let processor = Arc::clone(&self.processor);
            let websocket_connector = None;
            let remote_addr = conn.remote_addr();

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    Tunnel {
                        ca: Arc::clone(&ca),
                        client: client.clone(),
                        remote_addr,
                        websocket_connector: websocket_connector.clone(),
                        transporter: transporter.clone().unwrap(),
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
                scenario: "proxy start",
            })
    }
}
