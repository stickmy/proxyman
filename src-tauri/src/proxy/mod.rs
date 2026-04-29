use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use serde_json::json;
use tauri::{
    async_runtime::{self, Mutex},
    AppHandle, Emitter, Manager, Runtime, State,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    task::JoinHandle as TokioJoinHandle,
    time::{sleep, Duration},
};

use crate::{
    app_conf, ca::Ssl, commands,
    core_api::CORE_API_VERSION,
    core_api::{events as core_events, proxy as proxy_api},
    storage::SessionEventStore,
};

use self::service::ProxyService;

mod decoder;
mod rewind;
mod service;
mod sse;
mod tunnel;

type ProxyRuntime = (
    oneshot::Sender<()>,
    mpsc::Sender<commands::processor::ProcessorChannelMessage>,
    tauri::async_runtime::JoinHandle<()>,
    tauri::async_runtime::JoinHandle<()>,
    tauri::async_runtime::JoinHandle<()>,
    u16,
);
pub(crate) type ProxyState = Mutex<Option<ProxyRuntime>>;

type StandaloneProxyRuntime = (
    oneshot::Sender<()>,
    TokioJoinHandle<()>,
    TokioJoinHandle<()>,
    TokioJoinHandle<()>,
    TokioJoinHandle<()>,
    u16,
);

pub struct StandaloneProxyController {
    runtime: Mutex<Option<StandaloneProxyRuntime>>,
    core_event_tx: Option<mpsc::Sender<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandaloneProxyStatus {
    pub running: bool,
    pub port: Option<u16>,
}

impl StandaloneProxyController {
    pub fn new() -> Self {
        Self {
            runtime: Mutex::new(None),
            core_event_tx: None,
        }
    }

    pub fn with_core_event_tx(core_event_tx: mpsc::Sender<String>) -> Self {
        Self {
            runtime: Mutex::new(None),
            core_event_tx: Some(core_event_tx),
        }
    }

    pub async fn start(&self, port: u16) -> Result<StandaloneProxyStatus, String> {
        if port == 0 {
            return Err("Proxy port must be greater than 0".to_string());
        }

        {
            let runtime = self.runtime.lock().await;
            if let Some(runtime) = runtime.as_ref() {
                if runtime.5 == port {
                    return Ok(StandaloneProxyStatus {
                        running: true,
                        port: Some(port),
                    });
                }

                return Err(format!("proxy is already running on port {}", runtime.5));
            }
        }

        if !check_port_available(port).await {
            return Err(format!("port {} was occupied", port));
        }

        let addr: SocketAddr = ([127, 0, 0, 1], port).into();
        let (transporter_tx, mut transporter_recv) = mpsc::channel(200);
        let (typed_transporter_tx, mut typed_transporter_recv) = mpsc::channel(200);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (processor, _processor_tx, processor_receiver) = commands::processor::init();
        let processor_thread = tokio::spawn(processor_receiver);
        let ca = Arc::new(
            Ssl::load_or_generate(app_conf::app_ca_cert_file(), app_conf::app_ca_key_file())
                .map_err(|err| err.to_string())?,
        );

        let proxy_thread = tokio::spawn(async move {
            if let Err(err) = ProxyService::with_ca_and_typed_events(
                addr,
                Some(transporter_tx),
                Some(typed_transporter_tx),
                Arc::clone(&processor),
                ca,
            )
            .start(async move {
                let _ = shutdown_rx.await;
            })
            .await
            {
                log::error!("Running standalone proxy on {:?}, error: {}", addr, err);
            }
        });
        let core_event_tx = self.core_event_tx.clone();
        let legacy_events_thread = tokio::spawn(async move {
            while let Some(exchange) = transporter_recv.recv().await {
                match core_events::legacy_event_to_core_events(&exchange) {
                    Ok(core_events) => {
                        for core_event in core_events {
                            send_standalone_core_event(core_event_tx.as_ref(), core_event).await;
                        }
                    }
                    Err(err) => log::error!("Map standalone typed proxy event failed: {err}"),
                }
            }
        });
        let core_event_tx = self.core_event_tx.clone();
        let typed_events_thread = tokio::spawn(async move {
            while let Some(core_event) = typed_transporter_recv.recv().await {
                send_standalone_core_event(core_event_tx.as_ref(), core_event).await;
            }
        });

        if let Err(err) = wait_for_port_listening(port).await {
            drop(shutdown_tx);
            proxy_thread.abort();
            processor_thread.abort();
            legacy_events_thread.abort();
            typed_events_thread.abort();
            return Err(err);
        }

        let mut runtime = self.runtime.lock().await;
        runtime.replace((
            shutdown_tx,
            proxy_thread,
            processor_thread,
            legacy_events_thread,
            typed_events_thread,
            port,
        ));

        Ok(StandaloneProxyStatus {
            running: true,
            port: Some(port),
        })
    }

    pub async fn start_first_available(
        &self,
        preferred_port: u16,
    ) -> Result<StandaloneProxyStatus, String> {
        if preferred_port == 0 {
            return Err("Proxy port must be greater than 0".to_string());
        }

        {
            let runtime = self.runtime.lock().await;
            if let Some(runtime) = runtime.as_ref() {
                return Ok(StandaloneProxyStatus {
                    running: true,
                    port: Some(runtime.5),
                });
            }
        }

        let port = find_available_port(preferred_port)
            .await
            .ok_or_else(|| format!("no available proxy port found from {}", preferred_port))?;
        self.start(port).await
    }

    pub async fn available_port(&self, preferred_port: u16) -> Result<u16, String> {
        if preferred_port == 0 {
            return Err("Proxy port must be greater than 0".to_string());
        }

        {
            let runtime = self.runtime.lock().await;
            if let Some(runtime) = runtime.as_ref() {
                return Ok(runtime.5);
            }
        }

        find_available_port(preferred_port)
            .await
            .ok_or_else(|| format!("no available proxy port found from {}", preferred_port))
    }

    pub async fn stop(&self) -> Result<StandaloneProxyStatus, String> {
        let mut runtime = self.runtime.lock().await;
        if runtime.take().is_none() {
            return Err("proxy is not running".to_string());
        }

        Ok(StandaloneProxyStatus {
            running: false,
            port: None,
        })
    }

    pub async fn status(&self) -> StandaloneProxyStatus {
        let runtime = self.runtime.lock().await;
        StandaloneProxyStatus {
            running: runtime.is_some(),
            port: runtime.as_ref().map(|runtime| runtime.5),
        }
    }
}

async fn check_port_available(port: u16) -> bool {
    let v4_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
    if TcpStream::connect(v4_addr).await.is_ok() {
        return false;
    }

    TcpListener::bind(v4_addr).await.is_ok()
}

async fn find_available_port(preferred_port: u16) -> Option<u16> {
    for port in preferred_port..=u16::MAX {
        if check_port_available(port).await {
            return Some(port);
        }
    }

    None
}

async fn send_standalone_core_event(
    event_tx: Option<&mpsc::Sender<String>>,
    core_event: core_events::CoreEventEnvelope,
) {
    let Some(event_tx) = event_tx else {
        return;
    };

    let payload = match serde_json::to_value(core_event) {
        Ok(payload) => payload,
        Err(err) => {
            log::error!("Serialize standalone core event payload failed: {err}");
            return;
        }
    };
    let event = json!({
        "apiVersion": CORE_API_VERSION,
        "type": "proxyEvent",
        "payload": payload,
    });
    let event = match serde_json::to_string(&event) {
        Ok(event) => event,
        Err(err) => {
            log::error!("Serialize standalone core event envelope failed: {err}");
            return;
        }
    };

    if let Err(err) = event_tx.send(event).await {
        log::debug!("Send standalone core event to sidecar failed: {err}");
    }
}

async fn wait_for_port_listening(port: u16) -> Result<(), String> {
    let addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
    for _ in 0..40 {
        if TcpStream::connect(addr).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(25)).await;
    }

    Err(format!("proxy did not start listening on port {}", port))
}

pub fn set_proxy_state(app: &tauri::App) {
    app.manage(Mutex::new(None) as ProxyState);
}

#[tauri::command]
pub(crate) async fn start_proxy<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyState>,
    port: u16,
) -> Result<(), String> {
    start_proxy_runtime(app, &proxy, port).await
}

#[tauri::command]
pub(crate) async fn start_proxy_v1<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyState>,
    request: proxy_api::StartProxyRequest,
) -> Result<proxy_api::StartProxyResponse, String> {
    proxy_api::validate_start_request(&request)?;
    start_proxy_runtime(app, &proxy, request.port).await?;
    Ok(proxy_api::start_proxy_response(request.port))
}

async fn start_proxy_runtime<R: Runtime>(
    app: AppHandle<R>,
    proxy: &ProxyState,
    port: u16,
) -> Result<(), String> {
    let available = check_port_available(port).await;

    if !available {
        return Err(format!("port {} was occupied", port));
    }

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();

    let (transporter_tx, mut transporter_recv) = tokio::sync::mpsc::channel(200);
    let (typed_transporter_tx, mut typed_transporter_recv) = tokio::sync::mpsc::channel(200);
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // ------------------------------- Interceptors update channel -------------------------------
    let (processor, processor_tx, processor_receiver) = commands::processor::init();

    let processor_thread = async_runtime::spawn(processor_receiver);
    // ------------------------------- Interceptors update channel -------------------------------

    let ca = Arc::new(
        Ssl::load_or_generate(app_conf::app_ca_cert_file(), app_conf::app_ca_key_file())
            .map_err(|err| err.to_string())?,
    );

    let proxy_thread = async_runtime::spawn(async move {
        if let Err(e) = ProxyService::with_ca_and_typed_events(
            addr,
            Some(transporter_tx.clone()),
            Some(typed_transporter_tx.clone()),
            Arc::clone(&processor),
            ca,
        )
        .start(async move {
            let _ = shutdown_rx.await;
        })
        .await
        {
            log::error!("Running proxy on {:?}, error: {}", addr, e);
        }
    });

    let transporter_thread = async_runtime::spawn(async move {
        let mut session_store = SessionEventStore::open(app_conf::app_session_events_file())
            .map_err(|err| {
                log::error!("Session event store disabled: {err}");
                err
            })
            .ok();

        loop {
            tokio::select! {
                Some(exchange) = transporter_recv.recv() => {
                    if let Some(store) = session_store.as_mut() {
                        if let Err(err) = store.append(&exchange) {
                            log::error!("Persist proxy event failed: {err}");
                        }
                    }
                    match core_events::legacy_event_to_core_events(&exchange) {
                        Ok(core_events) => {
                            for core_event in core_events {
                                if let Err(err) = app.emit("proxy_event_v1", core_event) {
                                    log::error!("Emit typed proxy event failed: {err}");
                                }
                            }
                        }
                        Err(err) => log::error!("Map typed proxy event failed: {err}"),
                    }

                    if let Err(err) = app.emit("proxy_event", exchange) {
                        log::error!("Emit legacy proxy event failed: {err}");
                    }
                }
                Some(core_event) = typed_transporter_recv.recv() => {
                    if let Err(err) = app.emit("proxy_event_v1", core_event) {
                        log::error!("Emit typed proxy event failed: {err}");
                    }
                }
                else => break,
            }
        }
    });

    let mut proxy = proxy.lock().await;
    proxy.replace((
        shutdown_tx,
        processor_tx,
        proxy_thread,
        processor_thread,
        transporter_thread,
        port,
    ));

    Ok(())
}

#[tauri::command]
pub(crate) async fn stop_proxy(proxy: State<'_, ProxyState>) -> Result<(), String> {
    stop_proxy_state(&proxy).await
}

#[tauri::command]
pub(crate) async fn stop_proxy_v1(
    proxy: State<'_, ProxyState>,
    request: proxy_api::StopProxyRequest,
) -> Result<proxy_api::StopProxyResponse, String> {
    proxy_api::validate_stop_request(&request)?;
    stop_proxy_state(&proxy).await?;
    Ok(proxy_api::stop_proxy_response())
}

#[tauri::command]
pub(crate) async fn proxy_status(proxy: State<'_, ProxyState>) -> Result<bool, String> {
    let status = proxy_status_state(&proxy).await;
    Ok(status.running)
}

#[tauri::command]
pub(crate) async fn proxy_status_v1(
    proxy: State<'_, ProxyState>,
    request: proxy_api::ProxyStatusRequest,
) -> Result<proxy_api::ProxyStatusResponse, String> {
    proxy_api::validate_status_request(&request)?;
    let status = proxy_status_state(&proxy).await;
    Ok(proxy_api::proxy_status_response(status.running, status.port))
}

async fn stop_proxy_state(proxy: &ProxyState) -> Result<(), String> {
    let mut proxy = proxy.lock().await;
    stop_proxy_runtime(&mut proxy)
}

struct ProxyRuntimeStatus {
    running: bool,
    port: Option<u16>,
}

async fn proxy_status_state(proxy: &ProxyState) -> ProxyRuntimeStatus {
    let proxy = proxy.lock().await;
    let port = proxy.as_ref().map(|runtime| runtime.5);
    ProxyRuntimeStatus {
        running: proxy.is_some(),
        port,
    }
}

fn stop_proxy_runtime(proxy: &mut Option<ProxyRuntime>) -> Result<(), String> {
    if proxy.take().is_some() {
        Ok(())
    } else {
        Err("proxy is not running".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::events::{CoreEvent, CoreEventEnvelope, ExchangeStartedEvent};

    #[test]
    fn correctness_stop_proxy_when_not_running_returns_error() {
        let mut runtime: Option<ProxyRuntime> = None;

        assert_eq!(
            stop_proxy_runtime(&mut runtime),
            Err("proxy is not running".to_string())
        );
    }

    #[tokio::test]
    async fn sidecar_find_available_port_skips_bound_loopback_port() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("test listener should bind");
        let occupied_port = listener
            .local_addr()
            .expect("test listener should have a local address")
            .port();

        if occupied_port == u16::MAX {
            return;
        }

        let available_port = find_available_port(occupied_port)
            .await
            .expect("a later loopback port should be available");

        assert_ne!(available_port, occupied_port);
        assert!(available_port > occupied_port);
    }

    #[tokio::test]
    async fn sidecar_core_event_sink_wraps_proxy_event_envelope() {
        let (event_tx, mut event_rx) = mpsc::channel(1);
        let core_event = CoreEventEnvelope {
            api_version: CORE_API_VERSION,
            event: CoreEvent::ExchangeStarted(ExchangeStartedEvent {
                exchange_id: "exchange-1".to_string(),
                timestamp: 1,
                method: "GET".to_string(),
                uri: "http://127.0.0.1/".to_string(),
            }),
        };

        send_standalone_core_event(Some(&event_tx), core_event).await;

        let event = event_rx.recv().await.expect("sidecar event should be sent");
        let event: serde_json::Value =
            serde_json::from_str(&event).expect("sidecar event should be json");
        assert_eq!(event["apiVersion"], CORE_API_VERSION);
        assert_eq!(event["type"], "proxyEvent");
        assert_eq!(event["payload"]["event"]["kind"], "exchangeStarted");
        assert_eq!(
            event["payload"]["event"]["payload"]["exchangeId"],
            "exchange-1"
        );
    }

    #[tokio::test]
    async fn sidecar_available_port_returns_first_available_port() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("test listener should bind");
        let occupied_port = listener
            .local_addr()
            .expect("test listener should have a local address")
            .port();

        if occupied_port == u16::MAX {
            return;
        }

        let controller = StandaloneProxyController::new();
        let available_port = controller
            .available_port(occupied_port)
            .await
            .expect("a later loopback port should be available");

        assert_ne!(available_port, occupied_port);
        assert!(available_port > occupied_port);
    }
}
