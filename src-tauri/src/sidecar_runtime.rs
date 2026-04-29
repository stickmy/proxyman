use std::{path::PathBuf, sync::Arc};

use async_process::Command;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;

use serde_json::Value;

use crate::{
    app_conf,
    ca::RootCaMaterial,
    commands::{global_proxy, processor},
    core_api::{
        ca as ca_api, rules as rules_api,
        session::{
            ClearSessionRequest, CoreSessionApi, ExportSessionHarRequest,
            ImportSessionHarRequest, InMemorySessionStore, LoadSessionBodyRequest,
            ReplaySessionRequest, SearchSessionExchangesRequest,
        },
        system_proxy as system_proxy_api,
    },
    processors::persist::processor_persist::{
        create_pack_dir, delete_pack_dir, remove_processor_pack_status,
        write_processor_pack_status,
    },
};

pub use crate::proxy::{StandaloneProxyController, StandaloneProxyStatus};

pub struct SidecarRuntime {
    proxy: Arc<StandaloneProxyController>,
    session_store: InMemorySessionStore,
}

impl SidecarRuntime {
    pub fn new() -> Result<Self, String> {
        app_conf::init().map_err(|err| err.to_string())?;
        Ok(Self {
            proxy: Arc::new(StandaloneProxyController::new()),
            session_store: InMemorySessionStore::default(),
        })
    }

    pub fn new_with_core_event_tx(core_event_tx: mpsc::Sender<String>) -> Result<Self, String> {
        app_conf::init().map_err(|err| err.to_string())?;
        Ok(Self {
            proxy: Arc::new(StandaloneProxyController::with_core_event_tx(core_event_tx)),
            session_store: InMemorySessionStore::default(),
        })
    }

    pub fn proxy(&self) -> Arc<StandaloneProxyController> {
        Arc::clone(&self.proxy)
    }

    pub fn record_core_event_json(&self, event_json: &str) -> Result<(), String> {
        let event = serde_json::from_str::<Value>(event_json)
            .map_err(|err| format!("Parse sidecar event failed: {err}"))?;
        if event.get("type").and_then(Value::as_str) != Some("proxyEvent") {
            return Ok(());
        }

        let envelope = event
            .get("payload")
            .cloned()
            .ok_or_else(|| "Sidecar proxy event missing payload".to_string())
            .and_then(|payload| {
                serde_json::from_value(payload)
                    .map_err(|err| format!("Parse proxy event payload failed: {err}"))
            })?;
        self.session_store.apply_event(envelope)
    }

    pub async fn handle_session_command(
        &self,
        method: &str,
        params: Value,
    ) -> Option<Result<Value, String>> {
        let api = CoreSessionApi::new(self.session_store.clone());
        let result = match method {
            "sessions.search" => serde_json::from_value::<SearchSessionExchangesRequest>(params)
                .map_err(|err| format!("invalid sessions.search params: {err}"))
                .and_then(|request| api.search_exchanges(request))
                .and_then(to_json_value),
            "sessions.loadBody" => serde_json::from_value::<LoadSessionBodyRequest>(params)
                .map_err(|err| format!("invalid sessions.loadBody params: {err}"))
                .and_then(|request| api.load_body(request))
                .and_then(to_json_value),
            "sessions.clear" => serde_json::from_value::<ClearSessionRequest>(params)
                .map_err(|err| format!("invalid sessions.clear params: {err}"))
                .and_then(|request| api.clear(request))
                .and_then(to_json_value),
            "har.export" => serde_json::from_value::<ExportSessionHarRequest>(params)
                .map_err(|err| format!("invalid har.export params: {err}"))
                .and_then(|request| api.export_har(request))
                .and_then(to_json_value),
            "har.import" => serde_json::from_value::<ImportSessionHarRequest>(params)
                .map_err(|err| format!("invalid har.import params: {err}"))
                .and_then(|request| api.import_har(request))
                .and_then(to_json_value),
            "replay.send" => match serde_json::from_value::<ReplaySessionRequest>(params) {
                Ok(request) => api.replay(request).await.and_then(to_json_value),
                Err(err) => Err(format!("invalid replay.send params: {err}")),
            },
            _ => return None,
        };

        Some(result)
    }

    pub async fn handle_core_command(
        &self,
        method: &str,
        params: Value,
    ) -> Option<Result<Value, String>> {
        if let Some(result) = self.handle_session_command(method, params.clone()).await {
            return Some(result);
        }

        let result = match method {
            "ca.status" => handle_ca_status(params).await,
            "ca.install" => handle_ca_install(params).await,
            "systemProxy.enable" => {
                match parse_params::<system_proxy_api::EnableSystemProxyRequest>(method, params) {
                    Ok(request) => global_proxy::turn_on_global_proxy_v1(request)
                        .await
                        .and_then(to_json_value),
                    Err(err) => Err(err),
                }
            }
            "systemProxy.disable" => {
                match parse_params::<system_proxy_api::DisableSystemProxyRequest>(method, params) {
                    Ok(request) => global_proxy::turn_off_global_proxy_v1(request)
                        .await
                        .and_then(to_json_value),
                    Err(err) => Err(err),
                }
            }
            "systemProxy.status" => {
                match parse_params::<system_proxy_api::SystemProxyStatusRequest>(method, params) {
                    Ok(request) => global_proxy::system_proxy_status_v1(request)
                        .await
                        .and_then(to_json_value),
                    Err(err) => Err(err),
                }
            }
            "rules.listPacks" => parse_params::<rules_api::ListRulePacksRequest>(method, params)
                .and_then(processor::get_processor_packs_v1)
                .and_then(to_json_value),
            "rules.getPackRules" => {
                parse_params::<rules_api::GetRulePackRulesRequest>(method, params)
                    .and_then(processor::get_rule_pack_rules_v1)
                    .and_then(to_json_value)
            }
            "rules.validate" => parse_params::<rules_api::SaveRulePackRulesRequest>(method, params)
                .and_then(validate_rule_pack_rules_for_sidecar)
                .and_then(to_json_value),
            "rules.savePackRules" => {
                parse_params::<rules_api::SaveRulePackRulesRequest>(method, params)
                    .and_then(processor::save_rule_pack_rules_v1)
                    .and_then(to_json_value)
            }
            "rules.addPack" => parse_params::<rules_api::AddRulePackRequest>(method, params)
                .and_then(add_rule_pack_for_sidecar)
                .and_then(to_json_value),
            "rules.removePack" => parse_params::<rules_api::RemoveRulePackRequest>(method, params)
                .and_then(remove_rule_pack_for_sidecar)
                .and_then(to_json_value),
            "rules.updatePackStatus" => {
                parse_params::<rules_api::UpdateRulePackStatusRequest>(method, params)
                    .and_then(update_rule_pack_status_for_sidecar)
                    .and_then(to_json_value)
            }
            _ => return None,
        };

        Some(result)
    }
}

fn parse_params<T: DeserializeOwned>(method: &str, params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|err| format!("invalid {method} params: {err}"))
}

fn to_json_value<T: serde::Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|err| format!("Serialize core response failed: {err}"))
}

async fn handle_ca_status(params: Value) -> Result<Value, String> {
    let request = parse_params::<ca_api::CaStatusRequest>("ca.status", params)?;
    ca_api::validate_ca_status_request(&request)?;
    let installed = check_ca_installed_for_sidecar().await?;
    to_json_value(ca_api::ca_status_response(installed))
}

async fn handle_ca_install(params: Value) -> Result<Value, String> {
    let request = parse_params::<ca_api::CaInstallRequest>("ca.install", params)?;
    ca_api::validate_ca_install_request(&request)?;
    let installed = install_ca_for_sidecar().await?;
    to_json_value(ca_api::ca_install_response(installed))
}

fn add_rule_pack_for_sidecar(
    request: rules_api::AddRulePackRequest,
) -> Result<rules_api::RulePackMutationResponse, String> {
    rules_api::validate_add_rule_pack_request(&request)?;
    create_pack_dir(request.pack_name.as_str())
        .map_err(|err| format!("create pack dir failed: {err}"))?;
    write_processor_pack_status(request.pack_name.as_str(), request.enabled)
        .map_err(|err| format!("write pack status failed: {err}"))?;

    Ok(rules_api::rule_pack_mutation_response(true))
}

fn validate_rule_pack_rules_for_sidecar(
    request: rules_api::SaveRulePackRulesRequest,
) -> Result<rules_api::ValidateTypedRulesResponse, String> {
    let rules = rules_api::parse_save_rule_pack_rules_request(&request)?;
    Ok(rules_api::validate_typed_rules_response(&rules))
}

fn remove_rule_pack_for_sidecar(
    request: rules_api::RemoveRulePackRequest,
) -> Result<rules_api::RulePackMutationResponse, String> {
    rules_api::validate_remove_rule_pack_request(&request)?;
    delete_pack_dir(request.pack_name.as_str())
        .map_err(|err| format!("remove pack dir failed: {err}"))?;
    remove_processor_pack_status(request.pack_name.as_str())
        .map_err(|err| format!("remove pack status failed: {err}"))?;

    Ok(rules_api::rule_pack_mutation_response(true))
}

fn update_rule_pack_status_for_sidecar(
    request: rules_api::UpdateRulePackStatusRequest,
) -> Result<rules_api::RulePackMutationResponse, String> {
    rules_api::validate_update_rule_pack_status_request(&request)?;
    write_processor_pack_status(request.pack_name.as_str(), request.enabled)
        .map_err(|err| format!("write pack status failed: {err}"))?;

    Ok(rules_api::rule_pack_mutation_response(true))
}

fn ensure_ca_material_for_sidecar() -> Result<PathBuf, String> {
    let ca_path = app_conf::app_ca_cert_file();
    let key_path = app_conf::app_ca_key_file();
    RootCaMaterial::load_or_generate(&ca_path, &key_path).map_err(|err| err.to_string())?;

    Ok(ca_path)
}

async fn check_ca_installed_for_sidecar() -> Result<bool, String> {
    let ca_path = ensure_ca_material_for_sidecar()?;
    let output = Command::new("security")
        .arg("verify-cert")
        .arg("-c")
        .arg(ca_path.as_os_str())
        .output()
        .await;

    Ok(output
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map_or(false, |stdout| {
            stdout.contains("certificate verification successful")
        }))
}

async fn install_ca_for_sidecar() -> Result<bool, String> {
    let keychain = default_keychain_for_sidecar().await?;
    let ca_path = ensure_ca_material_for_sidecar()?;
    let output = Command::new("security")
        .arg("add-trusted-cert")
        .arg("-d")
        .arg("-r")
        .arg("trustRoot")
        .arg("-k")
        .arg(keychain.as_str())
        .arg(ca_path.as_os_str())
        .output()
        .await
        .map_err(|err| format!("Install CA certificate failed: {err}"))?;

    let stdout = String::from_utf8_lossy(output.stdout.as_slice());
    let stderr = String::from_utf8_lossy(output.stderr.as_slice());
    if output.status.success() && !stdout.contains("Error:") && !stderr.contains("Error:") {
        return Ok(true);
    }

    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    Err(format!("Install CA certificate failed: {detail}"))
}

async fn default_keychain_for_sidecar() -> Result<String, String> {
    let output = Command::new("security")
        .arg("default-keychain")
        .output()
        .await
        .map_err(|err| format!("Read default keychain failed: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(output.stderr.as_slice());
        return Err(format!("Read default keychain failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| format!("Read default keychain output failed: {err}"))?;
    let mut parts = stdout.split('"');
    parts.next();
    parts
        .next()
        .map(str::to_string)
        .ok_or_else(|| "Read default keychain output failed".to_string())
}
