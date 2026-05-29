use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{broadcast, mpsc},
};

use proxyman::sidecar_runtime::SidecarRuntime;

const CORE_API_VERSION: u16 = 1;
const DEFAULT_PROXY_PORT: u16 = 9000;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartProxyParams {
    #[serde(default = "default_host")]
    host: String,
    port: u16,
    #[serde(default)]
    find_available: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AvailablePortParams {
    #[serde(default = "default_host")]
    host: String,
    port: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AvailablePortResponse {
    api_version: u16,
    host: String,
    port: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProxyStatusResponse {
    api_version: u16,
    running: bool,
    state: &'static str,
    host: String,
    port: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SidecarEvent<'a> {
    api_version: u16,
    #[serde(rename = "type")]
    event_type: &'a str,
    payload: Value,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = SidecarConfig::from_args(env::args().skip(1).collect())?;
    prepare_socket(config.command_socket.as_path())?;
    if let Some(event_socket) = config.event_socket.as_ref() {
        prepare_socket(event_socket.as_path())?;
    }

    let (event_tx, _) = broadcast::channel::<String>(128);
    let (core_event_tx, mut core_event_rx) = mpsc::channel::<String>(512);
    let runtime = Arc::new(
        SidecarRuntime::new_with_core_event_tx(core_event_tx).map_err(io::Error::other)?,
    );
    let core_event_broadcast_tx = event_tx.clone();
    let core_event_runtime = Arc::clone(&runtime);
    let core_event_task = tokio::spawn(async move {
        while let Some(event) = core_event_rx.recv().await {
            if let Err(err) = core_event_runtime.record_core_event_json(event.as_str()) {
                eprintln!("record core event failed: {err}");
            }
            let _ = core_event_broadcast_tx.send(event);
        }
    });
    let command_task = serve_commands(config.command_socket.clone(), runtime, event_tx.clone());

    let event_task = if let Some(event_socket) = config.event_socket.clone() {
        Some(tokio::spawn(serve_events(event_socket, event_tx.clone())))
    } else {
        None
    };

    println!(
        "proxyman-sidecar ready command_socket={}",
        config.command_socket.display()
    );

    command_task.await?;
    core_event_task.abort();
    if let Some(event_task) = event_task {
        event_task.abort();
    }
    Ok(())
}

#[derive(Debug)]
struct SidecarConfig {
    command_socket: PathBuf,
    event_socket: Option<PathBuf>,
}

impl SidecarConfig {
    fn from_args(args: Vec<String>) -> io::Result<Self> {
        let mut command_socket = None;
        let mut event_socket = None;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--command-socket" => {
                    index += 1;
                    command_socket = args.get(index).map(PathBuf::from);
                }
                "--event-socket" => {
                    index += 1;
                    event_socket = args.get(index).map(PathBuf::from);
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown argument: {unknown}"),
                    ));
                }
            }
            index += 1;
        }

        let command_socket = command_socket.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--command-socket is required",
            )
        })?;

        Ok(Self {
            command_socket,
            event_socket,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: proxyman-sidecar --command-socket <path> [--event-socket <path>]"
    );
}

fn prepare_socket(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

async fn serve_commands(
    socket_path: PathBuf,
    runtime: Arc<SidecarRuntime>,
    event_tx: broadcast::Sender<String>,
) -> io::Result<()> {
    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (stream, _) = listener.accept().await?;
        let runtime = Arc::clone(&runtime);
        let event_tx = event_tx.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_command_stream(stream, runtime, event_tx).await {
                eprintln!("command stream failed: {err}");
            }
        });
    }
}

async fn handle_command_stream(
    stream: UnixStream,
    runtime: Arc<SidecarRuntime>,
    event_tx: broadcast::Sender<String>,
) -> io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_json_rpc_request(request, Arc::clone(&runtime), &event_tx).await,
            Err(err) => json_rpc_error(None, -32700, format!("parse error: {err}")),
        };
        writer.write_all(response.to_string().as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

async fn handle_json_rpc_request(
    request: JsonRpcRequest,
    runtime: Arc<SidecarRuntime>,
    event_tx: &broadcast::Sender<String>,
) -> Value {
    let id = request.id.clone();
    if let Some(result) =
        runtime
            .handle_core_command(request.method.as_str(), request.params.clone())
            .await
    {
        return match result {
            Ok(value) => json_rpc_result(id, value),
            Err(err) => json_rpc_error(id, -32602, err),
        };
    }

    let proxy = runtime.proxy();
    match request.method.as_str() {
        "diagnostics.status" => json_rpc_result(
            id,
            json!({
                "apiVersion": CORE_API_VERSION,
                "ok": true,
                "process": "proxyman-sidecar",
            }),
        ),
        "proxy.status" => {
            let status = proxy.status().await;
            json_rpc_result(id, proxy_status_response(status.running, status.port))
        }
        "proxy.availablePort" => {
            let params = match serde_json::from_value::<AvailablePortParams>(request.params) {
                Ok(params) => params,
                Err(err) => {
                    return json_rpc_error(
                        id,
                        -32602,
                        format!("invalid proxy.availablePort params: {err}"),
                    );
                }
            };
            if params.port == 0 {
                return json_rpc_error(id, -32602, "proxy.availablePort port must not be 0");
            }
            if params.host != default_host() {
                return json_rpc_error(
                    id,
                    -32602,
                    format!("proxy.availablePort host must be {}", default_host()),
                );
            }

            let port = match proxy.available_port(params.port).await {
                Ok(port) => port,
                Err(err) => return json_rpc_error(id, -32000, err),
            };
            json_rpc_result(id, available_port_response(port))
        }
        "proxy.start" => {
            let params = match serde_json::from_value::<StartProxyParams>(request.params) {
                Ok(params) => params,
                Err(err) => {
                    return json_rpc_error(
                        id,
                        -32602,
                        format!("invalid proxy.start params: {err}"),
                    );
                }
            };
            if params.port == 0 {
                return json_rpc_error(id, -32602, "proxy.start port must not be 0");
            }
            if params.host != default_host() {
                return json_rpc_error(
                    id,
                    -32602,
                    format!("proxy.start host must be {}", default_host()),
                );
            }

            let status = match if params.find_available {
                proxy.start_first_available(params.port).await
            } else {
                proxy.start(params.port).await
            } {
                Ok(status) => status,
                Err(err) => return json_rpc_error(id, -32000, err),
            };
            let response = proxy_status_response(status.running, status.port);
            emit_sidecar_event(event_tx, "proxy.started", response.clone());
            json_rpc_result(id, response)
        }
        "proxy.stop" => {
            let status = match proxy.stop().await {
                Ok(status) => status,
                Err(err) => return json_rpc_error(id, -32000, err),
            };
            let response = proxy_status_response(status.running, status.port);
            emit_sidecar_event(event_tx, "proxy.stopped", response.clone());
            json_rpc_result(id, response)
        }
        _ => json_rpc_error(
            id,
            -32601,
            format!("method not found: {}", request.method),
        ),
    }
}

fn available_port_response(port: u16) -> Value {
    serde_json::to_value(AvailablePortResponse {
        api_version: CORE_API_VERSION,
        host: default_host(),
        port,
    })
    .expect("available port response should serialize")
}

fn proxy_status_response(running: bool, port: Option<u16>) -> Value {
    serde_json::to_value(ProxyStatusResponse {
        api_version: CORE_API_VERSION,
        running,
        state: if running { "running" } else { "stopped" },
        host: default_host(),
        port: port.unwrap_or(DEFAULT_PROXY_PORT),
    })
    .expect("proxy status should serialize")
}

fn emit_sidecar_event(
    event_tx: &broadcast::Sender<String>,
    event_type: &'static str,
    payload: Value,
) {
    let event = serde_json::to_string(&SidecarEvent {
        api_version: CORE_API_VERSION,
        event_type,
        payload,
    })
    .expect("sidecar event should serialize");
    let _ = event_tx.send(event);
}

async fn serve_events(
    socket_path: PathBuf,
    event_tx: broadcast::Sender<String>,
) -> io::Result<()> {
    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(handle_event_stream(stream, event_tx.subscribe()));
    }
}

async fn handle_event_stream(
    mut stream: UnixStream,
    mut event_rx: broadcast::Receiver<String>,
) -> io::Result<()> {
    let connected = serde_json::to_string(&SidecarEvent {
        api_version: CORE_API_VERSION,
        event_type: "sidecar.connected",
        payload: json!({"ok": true}),
    })
    .expect("sidecar event should serialize");
    stream.write_all(connected.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    loop {
        match event_rx.recv().await {
            Ok(event) => {
                stream.write_all(event.as_bytes()).await?;
                stream.write_all(b"\n").await?;
                stream.flush().await?;
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

fn json_rpc_result(id: Option<Value>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "result": result,
    })
}

fn json_rpc_error(id: Option<Value>, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {
            "code": code,
            "message": message.into(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{
        body::to_bytes,
        service::{make_service_fn, service_fn},
        Body, Response, Server,
    };
    use std::net::SocketAddr;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn sidecar_sessions_search_load_and_clear_use_runtime_store() {
        let (event_tx, _) = broadcast::channel(1);
        let runtime = Arc::new(SidecarRuntime::new().expect("runtime should initialize"));
        runtime
            .record_core_event_json(
                json!({
                    "apiVersion": CORE_API_VERSION,
                    "type": "proxyEvent",
                    "payload": {
                        "apiVersion": CORE_API_VERSION,
                        "event": {
                            "kind": "requestHead",
                            "payload": {
                                "exchangeId": "exchange-1",
                                "timestamp": 1000,
                                "method": "POST",
                                "uri": "http://api.example.test/users",
                                "version": "HTTP/1.1",
                                "headers": [],
                                "capturedBody": {
                                    "preview": "{\"name\":\"proxyman\"}",
                                    "size": 19,
                                    "truncated": false,
                                    "bodyRef": null
                                }
                            }
                        }
                    }
                })
                .to_string()
                .as_str(),
            )
            .expect("event should record");

        let response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("request-1")),
                method: "sessions.search".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "filter": { "host": "api.example.test" }
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;

        assert!(response.get("error").is_none(), "{response}");
        assert_eq!(response["result"]["apiVersion"], CORE_API_VERSION);
        assert_eq!(
            response["result"]["exchanges"][0]["requestBodyRef"],
            "memory://exchange-1/request"
        );

        let response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("request-2")),
                method: "sessions.loadBody".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "bodyRef": "memory://exchange-1/request"
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;

        assert!(response.get("error").is_none(), "{response}");
        assert_eq!(response["result"]["body"], "{\"name\":\"proxyman\"}");

        let response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("request-3")),
                method: "sessions.clear".to_string(),
                params: json!({ "apiVersion": CORE_API_VERSION }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;

        assert!(response.get("error").is_none(), "{response}");
        assert_eq!(response["result"]["apiVersion"], CORE_API_VERSION);
        assert_eq!(response["result"]["cleared"], 1);
    }

    #[tokio::test]
    async fn sidecar_har_export_import_round_trips_runtime_store() {
        let (event_tx, _) = broadcast::channel(1);
        let runtime = Arc::new(SidecarRuntime::new().expect("runtime should initialize"));
        runtime
            .record_core_event_json(
                json!({
                    "apiVersion": CORE_API_VERSION,
                    "type": "proxyEvent",
                    "payload": {
                        "apiVersion": CORE_API_VERSION,
                        "event": {
                            "kind": "requestHead",
                            "payload": {
                                "exchangeId": "exchange-1",
                                "timestamp": 1000,
                                "method": "POST",
                                "uri": "http://api.example.test/users?active=true",
                                "version": "HTTP/1.1",
                                "headers": [
                                    { "name": "content-type", "value": "application/json" }
                                ],
                                "capturedBody": {
                                    "preview": "{\"name\":\"proxyman\"}",
                                    "size": 19,
                                    "truncated": false,
                                    "bodyRef": null
                                }
                            }
                        }
                    }
                })
                .to_string()
                .as_str(),
            )
            .expect("request event should record");
        runtime
            .record_core_event_json(
                json!({
                    "apiVersion": CORE_API_VERSION,
                    "type": "proxyEvent",
                    "payload": {
                        "apiVersion": CORE_API_VERSION,
                        "event": {
                            "kind": "responseHead",
                            "payload": {
                                "exchangeId": "exchange-1",
                                "timestamp": 1400,
                                "uri": "http://api.example.test/users?active=true",
                                "status": 201,
                                "version": "HTTP/1.1",
                                "headers": [
                                    { "name": "content-type", "value": "application/json" }
                                ],
                                "capturedBody": {
                                    "preview": "{\"ok\":true}",
                                    "size": 11,
                                    "truncated": false,
                                    "bodyRef": null
                                }
                            }
                        }
                    }
                })
                .to_string()
                .as_str(),
            )
            .expect("response event should record");

        let export_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("export-1")),
                method: "har.export".to_string(),
                params: json!({ "apiVersion": CORE_API_VERSION }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;

        assert!(export_response.get("error").is_none(), "{export_response}");
        let har = export_response["result"]["har"].clone();
        assert_eq!(har["log"]["entries"][0]["request"]["method"], "POST");
        assert_eq!(
            har["log"]["entries"][0]["request"]["url"],
            "http://api.example.test/users?active=true"
        );
        assert_eq!(har["log"]["entries"][0]["response"]["status"], 201);
        assert_eq!(
            har["log"]["entries"][0]["response"]["content"]["text"],
            "{\"ok\":true}"
        );

        let clear_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("clear-1")),
                method: "sessions.clear".to_string(),
                params: json!({ "apiVersion": CORE_API_VERSION }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(clear_response.get("error").is_none(), "{clear_response}");
        assert_eq!(clear_response["result"]["cleared"], 1);

        let import_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("import-1")),
                method: "har.import".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "har": har
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(import_response.get("error").is_none(), "{import_response}");
        assert_eq!(import_response["result"]["apiVersion"], CORE_API_VERSION);
        assert_eq!(import_response["result"]["imported"], 1);

        let search_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("search-1")),
                method: "sessions.search".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "filter": { "host": "api.example.test" }
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(search_response.get("error").is_none(), "{search_response}");
        assert_eq!(search_response["result"]["exchanges"][0]["method"], "POST");
        assert_eq!(search_response["result"]["exchanges"][0]["status"], 201);

        let response_body_ref = search_response["result"]["exchanges"][0]["responseBodyRef"]
            .as_str()
            .expect("imported response body ref should exist")
            .to_string();
        let load_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("load-1")),
                method: "sessions.loadBody".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "bodyRef": response_body_ref
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(load_response.get("error").is_none(), "{load_response}");
        assert_eq!(load_response["result"]["body"], "{\"ok\":true}");
    }

    #[tokio::test]
    async fn sidecar_replay_send_replays_in_memory_request_with_edits() {
        let origin_addr = reserve_local_addr();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let make_service = make_service_fn(|_| async {
            Ok::<_, hyper::Error>(service_fn(|req: hyper::Request<Body>| async move {
                let method = req.method().to_string();
                let path = req.uri().path().to_string();
                let header = req
                    .headers()
                    .get("x-replay")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("missing")
                    .to_string();
                let body = to_bytes(req.into_body()).await.unwrap_or_default();
                let body = String::from_utf8_lossy(body.as_ref()).into_owned();

                Ok::<_, hyper::Error>(Response::new(Body::from(format!(
                    "{method} {path} {header} {body}"
                ))))
            }))
        });
        let origin = Server::bind(&origin_addr)
            .serve(make_service)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            });
        tokio::spawn(origin);

        let (event_tx, _) = broadcast::channel(1);
        let runtime = Arc::new(SidecarRuntime::new().expect("runtime should initialize"));
        runtime
            .record_core_event_json(
                json!({
                    "apiVersion": CORE_API_VERSION,
                    "type": "proxyEvent",
                    "payload": {
                        "apiVersion": CORE_API_VERSION,
                        "event": {
                            "kind": "requestHead",
                            "payload": {
                                "exchangeId": "exchange-1",
                                "timestamp": 1000,
                                "method": "GET",
                                "uri": format!("http://{origin_addr}/old"),
                                "version": "HTTP/1.1",
                                "headers": [
                                    { "name": "accept", "value": "text/plain" }
                                ],
                                "capturedBody": {
                                    "preview": "",
                                    "size": 0,
                                    "truncated": false,
                                    "bodyRef": null
                                }
                            }
                        }
                    }
                })
                .to_string()
                .as_str(),
            )
            .expect("request event should record");

        let response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("replay-1")),
                method: "replay.send".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "exchangeId": "exchange-1",
                    "edit": {
                        "method": "POST",
                        "uri": format!("http://{origin_addr}/new"),
                        "headers": {
                            "x-replay": "matched"
                        },
                        "body": "body-ok"
                    }
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;

        assert!(response.get("error").is_none(), "{response}");
        assert_eq!(response["result"]["apiVersion"], CORE_API_VERSION);
        assert_eq!(response["result"]["status"], 200);
        assert_eq!(response["result"]["body"], "POST /new matched body-ok");

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn sidecar_rejects_invalid_system_proxy_enable_before_networksetup() {
        let (event_tx, _) = broadcast::channel(1);
        let runtime = Arc::new(SidecarRuntime::new().expect("runtime should initialize"));

        let response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("system-proxy-1")),
                method: "systemProxy.enable".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "port": 0
                }),
            },
            runtime,
            &event_tx,
        )
        .await;

        assert_eq!(response["error"]["code"], -32602);
        assert_eq!(
            response["error"]["message"],
            "System proxy port must be greater than 0"
        );
    }

    #[tokio::test]
    async fn sidecar_rejects_invalid_system_proxy_status_before_networksetup() {
        let (event_tx, _) = broadcast::channel(1);
        let runtime = Arc::new(SidecarRuntime::new().expect("runtime should initialize"));

        let response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("system-proxy-status-1")),
                method: "systemProxy.status".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "host": "127.0.0.1",
                    "port": 0
                }),
            },
            runtime,
            &event_tx,
        )
        .await;

        assert_eq!(response["error"]["code"], -32602);
        assert_eq!(
            response["error"]["message"],
            "System proxy port must be greater than 0"
        );
    }

    #[tokio::test]
    async fn sidecar_rejects_invalid_ca_status_version_before_security_command() {
        let (event_tx, _) = broadcast::channel(1);
        let runtime = Arc::new(SidecarRuntime::new().expect("runtime should initialize"));

        let response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("ca-1")),
                method: "ca.status".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION + 1
                }),
            },
            runtime,
            &event_tx,
        )
        .await;

        assert_eq!(response["error"]["code"], -32602);
        assert_eq!(
            response["error"]["message"],
            format!("Unsupported core API version: {}", CORE_API_VERSION + 1)
        );
    }

    #[tokio::test]
    async fn sidecar_rejects_invalid_upstream_tls_status_version_before_settings_read() {
        let (event_tx, _) = broadcast::channel(1);
        let runtime = Arc::new(SidecarRuntime::new().expect("runtime should initialize"));

        let response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("upstream-tls-1")),
                method: "upstreamTls.status".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION + 1
                }),
            },
            runtime,
            &event_tx,
        )
        .await;

        assert_eq!(response["error"]["code"], -32602);
        assert_eq!(
            response["error"]["message"],
            format!("Unsupported core API version: {}", CORE_API_VERSION + 1)
        );
    }

    #[tokio::test]
    async fn sidecar_rules_pack_commands_use_stable_socket_api() {
        let (event_tx, _) = broadcast::channel(1);
        let runtime = Arc::new(SidecarRuntime::new().expect("runtime should initialize"));
        let pack_name = format!("sidecar-test-{}", uuid::Uuid::new_v4());
        let content = "# keep comment\nredirect GET https://api.example.com/* https://mock.local\n";

        let save_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("rules-save")),
                method: "rules.savePackRules".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "packName": pack_name,
                    "enabled": true,
                    "content": content
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(save_response.get("error").is_none(), "{save_response}");
        assert_eq!(save_response["result"]["changed"], true);

        let validate_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("rules-validate")),
                method: "rules.validate".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "packName": pack_name,
                    "enabled": true,
                    "content": content
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(validate_response.get("error").is_none(), "{validate_response}");
        assert_eq!(validate_response["result"]["valid"], true);
        assert_eq!(
            validate_response["result"]["evaluationOrder"]
                .as_array()
                .expect("evaluation order should be an array")
                .len(),
            1
        );

        let get_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("rules-get")),
                method: "rules.getPackRules".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "packName": pack_name
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(get_response.get("error").is_none(), "{get_response}");
        assert_eq!(get_response["result"]["packName"], pack_name);
        assert_eq!(get_response["result"]["enabled"], true);
        assert_eq!(get_response["result"]["content"], content);
        assert_eq!(
            get_response["result"]["rules"][0]["actions"][0]["kind"],
            "redirect"
        );

        let update_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("rules-update")),
                method: "rules.updatePackStatus".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "packName": pack_name,
                    "enabled": false
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(update_response.get("error").is_none(), "{update_response}");
        assert_eq!(update_response["result"]["changed"], true);

        let list_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("rules-list")),
                method: "rules.listPacks".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION
                }),
            },
            Arc::clone(&runtime),
            &event_tx,
        )
        .await;
        assert!(list_response.get("error").is_none(), "{list_response}");
        let listed_pack = list_response["result"]["packs"]
            .as_array()
            .expect("packs should be an array")
            .iter()
            .find(|pack| pack["packName"] == pack_name)
            .expect("saved pack should be listed");
        assert_eq!(listed_pack["enabled"], false);

        let remove_response = handle_json_rpc_request(
            JsonRpcRequest {
                id: Some(json!("rules-remove")),
                method: "rules.removePack".to_string(),
                params: json!({
                    "apiVersion": CORE_API_VERSION,
                    "packName": pack_name
                }),
            },
            runtime,
            &event_tx,
        )
        .await;
        assert!(remove_response.get("error").is_none(), "{remove_response}");
        assert_eq!(remove_response["result"]["changed"], true);
    }

    fn reserve_local_addr() -> SocketAddr {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("failed to reserve local port");
        let addr = listener.local_addr().expect("failed to read local addr");
        drop(listener);
        addr
    }
}
