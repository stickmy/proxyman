use async_process::Command;
use futures::future;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{app_conf, core_api::system_proxy as system_proxy_api};

pub async fn turn_on_global_proxy(port: String) -> Result<bool, String> {
    let services = list_network_services().await?;
    if !system_proxy_snapshot_path().exists() {
        let snapshot = capture_system_proxy_snapshot(&services).await?;
        write_system_proxy_snapshot(&snapshot)?;
    }

    set_global_proxy(&services, "127.0.0.1", port.as_str()).await
}

pub async fn turn_off_global_proxy() -> Result<bool, String> {
    let snapshot_path = system_proxy_snapshot_path();
    if snapshot_path.exists() {
        let snapshot = read_system_proxy_snapshot()?;
        restore_system_proxy_snapshot(&snapshot).await?;
        fs::remove_file(snapshot_path).map_err(|err| {
            log::error!("Remove system proxy snapshot failed, {err}");
            "Remove system proxy snapshot failed".to_string()
        })?;

        return Ok(true);
    }

    disable_global_proxy().await
}

pub async fn turn_on_global_proxy_v1(
    request: system_proxy_api::EnableSystemProxyRequest,
) -> Result<system_proxy_api::SystemProxyResponse, String> {
    system_proxy_api::validate_enable_system_proxy_request(&request)?;
    turn_on_global_proxy(request.port.to_string()).await?;
    Ok(system_proxy_api::system_proxy_response(true))
}

pub async fn turn_off_global_proxy_v1(
    request: system_proxy_api::DisableSystemProxyRequest,
) -> Result<system_proxy_api::SystemProxyResponse, String> {
    system_proxy_api::validate_disable_system_proxy_request(&request)?;
    turn_off_global_proxy().await?;
    Ok(system_proxy_api::system_proxy_response(false))
}

pub async fn system_proxy_status_v1(
    request: system_proxy_api::SystemProxyStatusRequest,
) -> Result<system_proxy_api::SystemProxyStatusResponse, String> {
    system_proxy_api::validate_system_proxy_status_request(&request)?;

    let services = list_network_services().await?;
    let mut statuses = Vec::with_capacity(services.len());
    for service in services {
        let (web, secure_web) = future::join(
            get_global_proxy_status(&service, false),
            get_global_proxy_status(&service, true),
        )
        .await;
        let bypass_domains = get_proxy_bypass_domains(&service).await?;

        statuses.push(system_proxy_api::SystemProxyServiceStatus {
            service,
            web: system_proxy_state(web?),
            secure_web: system_proxy_state(secure_web?),
            bypass_domains,
        });
    }

    Ok(system_proxy_api::system_proxy_status_response(
        statuses,
        request.host.as_deref(),
        request.port,
    ))
}

async fn disable_global_proxy() -> Result<bool, String> {
    let services = list_network_services().await?;
    set_global_proxy(&services, "", "0").await?;

    for service in services.iter() {
        let http = Command::new("networksetup")
            .arg("-setwebproxystate")
            .arg(service)
            .arg("off")
            .output();

        let https = Command::new("networksetup")
            .arg("-setsecurewebproxystate")
            .arg(service)
            .arg("off")
            .output();

        let (http_output, https_output) = future::join(http, https).await;

        if !command_succeeded(http_output) || !command_succeeded(https_output) {
            return Err(format!("Turn off by networksetup failed for {service}"));
        }
    }

    for service in services.iter() {
        let (http_state, https_state) = future::join(
            get_global_proxy_status(service, false),
            get_global_proxy_status(service, true),
        )
        .await;

        match (http_state, https_state) {
            (Ok(http_state), Ok(https_state))
                if !http_state.enabled && !https_state.enabled => {}
            _ => return Err(format!("Turn off by networksetup failed for {service}")),
        }
    }

    Ok(true)
}

async fn set_global_proxy(services: &[String], host: &str, port: &str) -> Result<bool, String> {
    if services.is_empty() {
        return Err("No active network services found".to_string());
    }

    for service in services.iter() {
        let http = Command::new("networksetup")
            .arg("-setwebproxy")
            .arg(service)
            .arg(host)
            .arg(port)
            .output();

        let https = Command::new("networksetup")
            .arg("-setsecurewebproxy")
            .arg(service)
            .arg(host)
            .arg(port)
            .output();

        let (http_output, https_output) = future::join(http, https).await;
        if !command_succeeded(http_output) || !command_succeeded(https_output) {
            return Err(format!("Turn on by networksetup failed for {service}"));
        }
    }

    for service in services.iter() {
        let (http_state, https_state) = future::join(
            get_global_proxy_status(service, false),
            get_global_proxy_status(service, true),
        )
        .await;

        match (http_state, https_state) {
            (Ok(http_state), Ok(https_state))
                if http_state.port == https_state.port
                    && http_state.port.as_deref() == Some(port)
                    && http_state.server == https_state.server
                    && http_state.server.as_deref() == Some(host) => {}
            _ => return Err(format!("Turn on by networksetup failed for {service}")),
        }
    }

    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct GlobalProxyState {
    pub enabled: bool,
    pub server: Option<String>,
    pub port: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ServiceProxySnapshot {
    service: String,
    web: GlobalProxyState,
    secure_web: GlobalProxyState,
    bypass_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SystemProxySnapshot {
    services: Vec<ServiceProxySnapshot>,
}

async fn capture_system_proxy_snapshot(services: &[String]) -> Result<SystemProxySnapshot, String> {
    let mut snapshots = Vec::with_capacity(services.len());

    for service in services.iter() {
        let (web, secure_web) = future::join(
            get_global_proxy_status(service, false),
            get_global_proxy_status(service, true),
        )
        .await;
        let bypass_domains = get_proxy_bypass_domains(service).await?;

        snapshots.push(ServiceProxySnapshot {
            service: service.clone(),
            web: web?,
            secure_web: secure_web?,
            bypass_domains,
        });
    }

    Ok(SystemProxySnapshot {
        services: snapshots,
    })
}

async fn restore_system_proxy_snapshot(snapshot: &SystemProxySnapshot) -> Result<(), String> {
    for service in snapshot.services.iter() {
        restore_proxy_state(&service.service, false, &service.web).await?;
        restore_proxy_state(&service.service, true, &service.secure_web).await?;
        restore_proxy_bypass_domains(&service.service, &service.bypass_domains).await?;
    }

    Ok(())
}

async fn restore_proxy_state(
    service: &str,
    secure: bool,
    state: &GlobalProxyState,
) -> Result<(), String> {
    let state_command = if secure {
        "-setsecurewebproxystate"
    } else {
        "-setwebproxystate"
    };

    if !state.enabled {
        let output = Command::new("networksetup")
            .arg(state_command)
            .arg(service)
            .arg("off")
            .output()
            .await;

        return command_succeeded(output)
            .then_some(())
            .ok_or_else(|| format!("Restore disabled proxy state failed for {service}"));
    }

    let server = state
        .server
        .as_deref()
        .ok_or_else(|| format!("Restore proxy state missing server for {service}"))?;
    let port = state
        .port
        .as_deref()
        .ok_or_else(|| format!("Restore proxy state missing port for {service}"))?;
    let set_command = if secure {
        "-setsecurewebproxy"
    } else {
        "-setwebproxy"
    };

    let set_output = Command::new("networksetup")
        .arg(set_command)
        .arg(service)
        .arg(server)
        .arg(port)
        .output()
        .await;

    if !command_succeeded(set_output) {
        return Err(format!("Restore proxy server failed for {service}"));
    }

    let state_output = Command::new("networksetup")
        .arg(state_command)
        .arg(service)
        .arg("on")
        .output()
        .await;

    command_succeeded(state_output)
        .then_some(())
        .ok_or_else(|| format!("Restore enabled proxy state failed for {service}"))
}

async fn restore_proxy_bypass_domains(
    service: &str,
    bypass_domains: &[String],
) -> Result<(), String> {
    let mut command = Command::new("networksetup");
    command.arg("-setproxybypassdomains").arg(service);
    for domain in bypass_domain_args(bypass_domains) {
        command.arg(domain);
    }

    command_succeeded(command.output().await)
        .then_some(())
        .ok_or_else(|| format!("Restore proxy bypass domains failed for {service}"))
}

fn system_proxy_snapshot_path() -> PathBuf {
    app_conf::app_system_proxy_snapshot_file()
}

fn write_system_proxy_snapshot(snapshot: &SystemProxySnapshot) -> Result<(), String> {
    write_system_proxy_snapshot_to_path(&system_proxy_snapshot_path(), snapshot)
}

fn write_system_proxy_snapshot_to_path(
    path: &Path,
    snapshot: &SystemProxySnapshot,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            log::error!("Create system proxy snapshot dir failed, {err}");
            "Create system proxy snapshot dir failed".to_string()
        })?;
    }

    let content = serde_json::to_vec(snapshot).map_err(|err| {
        log::error!("Serialize system proxy snapshot failed, {err}");
        "Serialize system proxy snapshot failed".to_string()
    })?;

    fs::write(path, content).map_err(|err| {
        log::error!("Write system proxy snapshot failed, {err}");
        "Write system proxy snapshot failed".to_string()
    })
}

fn read_system_proxy_snapshot() -> Result<SystemProxySnapshot, String> {
    read_system_proxy_snapshot_from_path(&system_proxy_snapshot_path())
}

fn read_system_proxy_snapshot_from_path(path: &Path) -> Result<SystemProxySnapshot, String> {
    let content = fs::read(path).map_err(|err| {
        log::error!("Read system proxy snapshot failed, {err}");
        "Read system proxy snapshot failed".to_string()
    })?;

    serde_json::from_slice(content.as_slice()).map_err(|err| {
        log::error!("Deserialize system proxy snapshot failed, {err}");
        "Deserialize system proxy snapshot failed".to_string()
    })
}

async fn list_network_services() -> Result<Vec<String>, String> {
    let child = Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output()
        .await;

    let output = child
        .map_err(|e| {
            log::error!("List network services failed, {e}");

            "List network services failed".to_string()
        })
        .and_then(|out| {
            String::from_utf8(out.stdout)
                .map_err(|_| "Read network service output error".to_string())
        })?;

    Ok(parse_network_services(output.as_str()))
}

fn parse_network_services(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("An asterisk"))
        .filter(|line| !line.starts_with('*'))
        .map(ToOwned::to_owned)
        .collect()
}

async fn get_global_proxy_status(
    service: &str,
    secure: bool,
) -> Result<GlobalProxyState, String> {
    let child = Command::new("networksetup")
        .arg(if secure {
            "-getsecurewebproxy"
        } else {
            "-getwebproxy"
        })
        .arg(service)
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

    Ok(parse_global_proxy_status(output.as_str()))
}

async fn get_proxy_bypass_domains(service: &str) -> Result<Vec<String>, String> {
    let child = Command::new("networksetup")
        .arg("-getproxybypassdomains")
        .arg(service)
        .output()
        .await;

    let output = child
        .map_err(|e| {
            log::error!("Get proxy bypass domains failed, {e}");

            "Get proxy bypass domains failed".to_string()
        })
        .and_then(|out| {
            String::from_utf8(out.stdout)
                .map_err(|_| "Read proxy bypass domain output error".to_string())
        })?;

    Ok(parse_proxy_bypass_domains(output.as_str()))
}

fn parse_global_proxy_status(output: &str) -> GlobalProxyState {
    let mut state = GlobalProxyState::default();

    for line in output.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();

            if key == "enabled" {
                state.enabled = value.eq_ignore_ascii_case("yes");
                if value.eq_ignore_ascii_case("no") {
                    break;
                }
            } else if key == "server" {
                state.server = Some(value.to_string());
            } else if key == "port" {
                state.port = Some(value.to_string());
            }
        }
    }

    state
}

fn system_proxy_state(state: GlobalProxyState) -> system_proxy_api::SystemProxyState {
    system_proxy_api::SystemProxyState {
        enabled: state.enabled,
        server: state.server,
        port: state.port,
    }
}

fn parse_proxy_bypass_domains(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("There aren't any bypass domains set on"))
        .map(ToOwned::to_owned)
        .collect()
}

fn bypass_domain_args(domains: &[String]) -> Vec<String> {
    if domains.is_empty() {
        vec!["Empty".to_string()]
    } else {
        domains.to_vec()
    }
}

fn command_succeeded(output: Result<async_process::Output, std::io::Error>) -> bool {
    output.map(|output| output.status.success()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_proxy_parse_enabled_network_services() {
        let output = "\
An asterisk (*) denotes that a network service is disabled.\n\
Wi-Fi\n\
USB 10/100/1000 LAN\n\
*Bluetooth PAN\n\
\n";

        assert_eq!(
            parse_network_services(output),
            vec![
                "Wi-Fi".to_string(),
                "USB 10/100/1000 LAN".to_string(),
            ]
        );
    }

    #[test]
    fn system_proxy_parse_enabled_proxy_state() {
        let output = "\
Enabled: Yes\n\
Server: 127.0.0.1\n\
Port: 8888\n\
Authenticated Proxy Enabled: 0\n";

        assert_eq!(
            parse_global_proxy_status(output),
            GlobalProxyState {
                enabled: true,
                server: Some("127.0.0.1".to_string()),
                port: Some("8888".to_string()),
            }
        );
    }

    #[test]
    fn system_proxy_parse_disabled_proxy_state_without_stale_server() {
        let output = "\
Enabled: No\n\
Server: stale.local\n\
Port: 8888\n\
Authenticated Proxy Enabled: 0\n";

        assert_eq!(
            parse_global_proxy_status(output),
            GlobalProxyState {
                enabled: false,
                server: None,
                port: None,
            }
        );
    }

    #[test]
    fn system_proxy_parse_bypass_domains() {
        let output = "\
*.local\n\
169.254/16\n\
localhost\n";

        assert_eq!(
            parse_proxy_bypass_domains(output),
            vec![
                "*.local".to_string(),
                "169.254/16".to_string(),
                "localhost".to_string(),
            ]
        );
    }

    #[test]
    fn system_proxy_parse_empty_bypass_domain_message() {
        let output = "There aren't any bypass domains set on Wi-Fi.\n";

        assert!(parse_proxy_bypass_domains(output).is_empty());
    }

    #[test]
    fn system_proxy_empty_bypass_domains_restore_with_empty_marker() {
        assert_eq!(bypass_domain_args(&[]), vec!["Empty".to_string()]);
    }

    #[test]
    fn system_proxy_snapshot_round_trips_to_disk() {
        let dir = std::env::temp_dir().join(format!("proxyman-proxy-{}", uuid::Uuid::new_v4()));
        let path = dir.join("system_proxy_snapshot.json");
        let snapshot = SystemProxySnapshot {
            services: vec![ServiceProxySnapshot {
                service: "Wi-Fi".to_string(),
                web: GlobalProxyState {
                    enabled: true,
                    server: Some("corp.proxy".to_string()),
                    port: Some("8080".to_string()),
                },
                secure_web: GlobalProxyState {
                    enabled: false,
                    server: None,
                    port: None,
                },
                bypass_domains: vec!["*.local".to_string(), "169.254/16".to_string()],
            }],
        };

        write_system_proxy_snapshot_to_path(&path, &snapshot).expect("failed to write snapshot");
        assert_eq!(
            read_system_proxy_snapshot_from_path(&path).expect("failed to read snapshot"),
            snapshot
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
