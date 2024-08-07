use serde::{Deserialize, Serialize};
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    sync::Arc,
};
use tauri::{
    async_runtime::{self, Mutex},
    AppHandle, Emitter, Manager, Runtime, State,
};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};

use crate::commands;

use self::service::ProxyService;

mod decoder;
mod rewind;
mod service;
mod tunnel;

pub(crate) type ProxyState = Mutex<
    Option<(
        oneshot::Sender<()>,
        mpsc::Sender<commands::processor::ProcessorChannelMessage>,
        tauri::async_runtime::JoinHandle<()>,
        tauri::async_runtime::JoinHandle<()>,
        tauri::async_runtime::JoinHandle<()>,
    )>,
>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProxyStartResult {
    pub success: bool,
    pub reason: Option<String>,
}

async fn check_port_available(port: u16) -> bool {
    let v4_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
    let v6_addr = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0);
    TcpListener::bind(v4_addr).await.is_ok() && TcpListener::bind(v6_addr).await.is_ok()
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
    let available = check_port_available(port).await;

    if !available {
        return Err(format!("port {} was occupied", port));
    }

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();

    let (transporter_tx, mut transporter_recv) = tokio::sync::mpsc::channel(200);
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // ------------------------------- Interceptors update channel -------------------------------
    let (processor, processor_tx, processor_receiver) = commands::processor::init();

    let processor_thread = async_runtime::spawn(processor_receiver);
    // ------------------------------- Interceptors update channel -------------------------------

    let proxy_thread = async_runtime::spawn(async move {
        if let Err(e) =
            ProxyService::new(addr, Some(transporter_tx.clone()), Arc::clone(&processor))
                .start(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        {
            log::error!("Running proxy on {:?}, error: {}", addr, e);
        }
    });

    let transporter_thread = async_runtime::spawn(async move {
        while let Some(exchange) = transporter_recv.recv().await {
            app.emit("proxy_event", exchange).unwrap();
        }
    });

    let mut proxy = proxy.lock().await;
    proxy.replace((
        shutdown_tx,
        processor_tx,
        proxy_thread,
        processor_thread,
        transporter_thread,
    ));

    Ok(())
}

#[tauri::command]
pub(crate) async fn stop_proxy(proxy: State<'_, ProxyState>) -> Result<(), String> {
    let mut proxy = proxy.lock().await;
    assert!(proxy.is_some());
    proxy.take();
    Ok(())
}

#[tauri::command]
pub(crate) async fn proxy_status(proxy: State<'_, ProxyState>) -> Result<bool, String> {
    Ok(proxy.lock().await.is_some())
}
