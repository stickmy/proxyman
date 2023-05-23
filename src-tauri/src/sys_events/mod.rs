use tauri::{AppHandle, RunEvent, Runtime};

use crate::commands::global_proxy::turn_off_global_proxy;

#[allow(clippy::single_match)]
pub fn handle_sys_events<R: Runtime>(_app_handler: &AppHandle<R>, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { .. } => {
            tauri::async_runtime::block_on(async {
                if let Err(e) = turn_off_global_proxy().await {
                    log::error!("App exits with cleaup failed - turn off proxy, {}", e);
                }
            });
        }
        _ => {}
    }
}
