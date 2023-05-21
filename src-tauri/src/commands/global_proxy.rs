use async_process::Command;
use futures::future;
use serde::{Deserialize, Serialize};

#[tauri::command]
pub async fn turn_on_global_proxy(port: String) -> Result<bool, String> {
    let http = Command::new("networksetup")
        .arg("-setwebproxy")
        .arg("Wi-Fi")
        .arg("127.0.0.1")
        .arg(&port)
        .output();

    let https = Command::new("networksetup")
        .arg("-setsecurewebproxy")
        .arg("Wi-Fi")
        .arg("127.0.0.1")
        .arg(&port)
        .output();

    let (http_output, https_output) = future::join(http, https).await;
    if http_output.is_ok() && https_output.is_ok() {
        let (http_state, https_state) = future::join(
            get_global_proxy_status(false),
            get_global_proxy_status(true),
        )
        .await;

        if let Ok(http_state) = http_state {
            if let Ok(https_state) = https_state {
                return Ok(http_state.port == https_state.port && http_state.port == Some(port));
            }
        }
    }

    Err("Turn on by networksetup failed".to_string())
}

#[tauri::command]
pub async fn turn_off_global_proxy() -> Result<bool, String> {
    let http = Command::new("networksetup")
        .arg("-setwebproxystate")
        .arg("Wi-Fi")
        .arg("off")
        .output();

    let https = Command::new("networksetup")
        .arg("-setsecurewebproxystate")
        .arg("Wi-Fi")
        .arg("off")
        .output();

    let (http_output, https_output) = future::join(http, https).await;

    if http_output.is_ok() && https_output.is_ok() {
        let (http_state, https_state) = future::join(
            get_global_proxy_status(false),
            get_global_proxy_status(true),
        )
        .await;

        if let Ok(http_state) = http_state {
            if let Ok(https_state) = https_state {
                return Ok(!http_state.enabled && !https_state.enabled);
            }
        }
    }

    Err("Turn off by networksetup failed".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct GlobalProxyState {
    pub enabled: bool,
    pub server: Option<String>,
    pub port: Option<String>,
}

async fn get_global_proxy_status(secure: bool) -> Result<GlobalProxyState, String> {
    let child = Command::new("networksetup")
        .arg(if secure {
            "-getsecurewebproxy"
        } else {
            "-getwebproxy"
        })
        .arg("Wi-Fi")
        .output()
        .await;

    let output = child
        .map_err(|e| {
            log::error!("Get web proxy state, {e}");

            "Get web proxy state error".to_string()
        })
        .and_then(|out| {
            String::from_utf8(out.stdout)
                .map_err(|_| "Read web proxy state output error".to_string())
        })?;

    let mut state = GlobalProxyState::default();
    let lines = output.split('\n');

    for line in lines.into_iter() {
        let mut parts = line.split(':');

        let key = parts.next().map(|k| k.trim().to_lowercase());
        let value = parts.next().map(|v| v.trim().to_lowercase());

        if let Some(key) = key {
            if key == "enabled" {
                match value {
                    Some(value) => {
                        state.enabled = value == "yes";
                        if value == "no" {
                            break;
                        }
                    }
                    None => {
                        break;
                    }
                }
            } else if key == "server" {
                state.server = value;
            } else if key == "port" {
                state.port = value;
            }
        }
    }

    Ok(state)
}
