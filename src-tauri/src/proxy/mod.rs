use serde::{Deserialize, Serialize};
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    sync::Arc,
};
use tauri::{
    async_runtime::{self, Mutex},
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime, State,
};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};

use crate::commands;

use self::service::ProxyService;

mod decoder;
pub mod processor;
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
            app.emit_all("proxy_event", exchange).unwrap();
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
async fn stop_proxy(proxy: State<'_, ProxyState>) -> Result<(), String> {
    let mut proxy = proxy.lock().await;
    assert!(proxy.is_some());
    proxy.take();
    Ok(())
}

#[tauri::command]
async fn proxy_status(proxy: State<'_, ProxyState>) -> Result<bool, String> {
    Ok(proxy.lock().await.is_some())
}

async fn check_port_available(port: u16) -> bool {
    let v4_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
    let v6_addr = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0);
    TcpListener::bind(v4_addr).await.is_ok() && TcpListener::bind(v6_addr).await.is_ok()
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("proxy")
        .setup(|app_handle| {
            app_handle.manage(Mutex::new(None) as ProxyState);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_proxy,
            stop_proxy,
            proxy_status,
            commands::ca::check_cert_installed,
            commands::ca::install_cert,
            commands::processor::set_processor,
            commands::processor::get_processor_content,
            commands::processor::remove_response_mapping,
            commands::processor::add_processor_pack,
            commands::processor::remove_processor_pack,
            commands::processor::update_processor_pack_status,
            commands::global_proxy::turn_on_global_proxy,
            commands::global_proxy::turn_off_global_proxy,
            commands::app_setting::set_app_setting,
            commands::app_setting::get_app_setting,
        ])
        .build()
}
