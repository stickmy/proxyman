use std::{future::Future, sync::Arc};

use serde::Serialize;
use tauri::{async_runtime::Mutex, State};
use tokio::sync::mpsc::{self, Sender};

use crate::core_api::rules as rules_api;
use crate::processors::parser::ProcessorRuleParser;
use crate::processors::persist::processor_persist::{
    create_pack_dir, delete_pack_dir, read_rule_pack_rules, remove_legacy_processor_files,
    remove_processor_pack_status, rule_pack_enabled, write_processor_pack_status,
    write_rule_pack_rules,
};
use crate::processors::processor_pack::ProcessorPack;
use crate::{
    processors::{
        http_processor::{
            delay::{RequestDelayProcessor, RequestDelayRule},
            redirect::RequestRedirectProcessor,
            request_header::RequestHeaderProcessor,
            response::ResponseProcessor,
            response_header::ResponseHeaderProcessor,
            HttpProcessor,
        },
        persist::processor_persist::{
            read_processor, read_processors_from_appdir, write_processor,
        },
        processor_id::ProcessorID,
    },
    proxy::ProxyState,
};

pub(crate) enum ProcessorChannelMessage {
    // processor message
    Redirect(String, Vec<[String; 2]>),
    Delay(String, RequestDelayRule),
    RequestHeader(String, Vec<[String; 4]>),
    Response(String, Vec<[String; 2]>),
    ResponseHeader(String, Vec<[String; 3]>),
    // packs message
    AddPack(String, bool),
    RemovePack(String),
    UpdatePackStatus(String, bool),
}

pub(crate) fn init() -> (
    Arc<Mutex<HttpProcessor>>,
    Sender<ProcessorChannelMessage>,
    impl Future<Output = ()>,
) {
    let packs = read_processors_from_appdir();

    let processor = Arc::new(Mutex::new(HttpProcessor::new(packs)));

    let (tx, mut rx) = mpsc::channel::<ProcessorChannelMessage>(1);

    let processor_setter = Arc::clone(&processor);
    let receiver = async move {
        log::debug!("Start listening ProcessorChannelMessage ...");

        while let Some(message) = rx.recv().await {
            let mut processor_setter = processor_setter.lock().await;

            match message {
                ProcessorChannelMessage::AddPack(pack_name, enable) => {
                    processor_setter.add_pack(ProcessorPack::new(pack_name, enable));
                }
                ProcessorChannelMessage::RemovePack(pack_name) => {
                    processor_setter.remove_pack(pack_name);
                }
                ProcessorChannelMessage::UpdatePackStatus(pack_name, status) => {
                    if status {
                        processor_setter.enable_pack(pack_name);
                    } else {
                        processor_setter.disable_pack(pack_name);
                    }
                }
                ProcessorChannelMessage::Redirect(pack_name, mapping) => {
                    if let Some(redirect) = processor_setter.get_redirect_mut(pack_name) {
                        redirect.set_redirects_mapping(mapping);
                    }
                }
                ProcessorChannelMessage::Delay(pack_name, mappings) => {
                    if let Some(delay) = processor_setter.get_delay_mut(pack_name) {
                        delay.set_delay_mapping(mappings);
                    }
                }
                ProcessorChannelMessage::Response(pack_name, mapping) => {
                    if let Some(response) = processor_setter.get_response_mut(pack_name) {
                        response.set_mapping(mapping);
                    }
                }
                ProcessorChannelMessage::RequestHeader(pack_name, mapping) => {
                    if let Some(request_header) =
                        processor_setter.get_request_header_mut(pack_name)
                    {
                        request_header.set_mapping(mapping);
                    }
                }
                ProcessorChannelMessage::ResponseHeader(pack_name, mapping) => {
                    if let Some(response_header) =
                        processor_setter.get_response_header_mut(pack_name)
                    {
                        response_header.set_mapping(mapping);
                    }
                }
            }
        }
    };

    (Arc::clone(&processor), tx, receiver)
}

#[tauri::command]
pub(crate) async fn set_processor(
    state: State<'_, ProxyState>,
    mode: String,
    pack_name: String,
    content: String,
) -> Result<bool, String> {
    log::debug!(
        "Set Processor - pack_name: {}, mode: {}, content: {}",
        pack_name,
        mode,
        content
    );

    let processor_id = ProcessorID::try_from(mode)?;

    let save_ret = write_processor(processor_id, content.as_str(), pack_name.as_str());

    if let Err(e) = save_ret {
        log::error!(
            "Writing processor as file - id: {}, content: {}, error: {}",
            processor_id,
            content,
            e
        );
        return Err("save processor failed".to_string());
    }

    let mut state = state.lock().await;

    if state.is_some() {
        let msg = {
            match processor_id {
                ProcessorID::REDIRECT => Some(ProcessorChannelMessage::Redirect(
                    pack_name,
                    RequestRedirectProcessor::parse_rule(content.as_str()),
                )),
                ProcessorID::DELAY => Some(ProcessorChannelMessage::Delay(
                    pack_name,
                    RequestDelayProcessor::parse_rule(content.as_str()),
                )),
                ProcessorID::RESPONSE => Some(ProcessorChannelMessage::Response(
                    pack_name,
                    ResponseProcessor::parse_rule(content.as_str()),
                )),
                ProcessorID::REQUEST_HEADER => Some(ProcessorChannelMessage::RequestHeader(
                    pack_name,
                    RequestHeaderProcessor::parse_rule(content.as_str()),
                )),
                ProcessorID::RESPONSE_HEADER => Some(ProcessorChannelMessage::ResponseHeader(
                    pack_name,
                    ResponseHeaderProcessor::parse_rule(content.as_str()),
                )),
                _ => return Err("Unsupport processor".into()),
            }
        };

        match msg {
            Some(msg) => match state.as_mut() {
                Some((_, processor, _, _, _, _)) => {
                    if let Err(e) = processor.send(msg).await {
                        log::error!("Set Processor failed: {e}");
                        return Err(format!("Set Processor failed: {e}"));
                    }
                }
                None => {
                    log::error!("Set Processor failed: No ProcessorChannelMessage Sender found");
                    return Err("No ProcessorChannelMessage Sender found".to_string());
                }
            },
            None => {
                log::error!("Parsing Processor rule failed");
                return Err("Parsing Processor rule failed".to_string());
            }
        }
    }

    Ok(true)
}

#[tauri::command]
pub(crate) async fn set_processor_v1(
    state: State<'_, ProxyState>,
    request: rules_api::SetRuleContentRequest,
) -> Result<rules_api::SetRuleContentResponse, String> {
    rules_api::validate_set_rule_content_request(&request)?;
    let saved = set_processor(
        state,
        request.kind.legacy_processor_mode().to_string(),
        request.pack_name,
        request.content,
    )
    .await?;

    Ok(rules_api::set_rule_content_response(saved))
}

#[tauri::command]
pub fn get_processor_content(mode: String, pack_name: String) -> Result<String, String> {
    let processor_id = ProcessorID::try_from(mode)?;

    read_processor(processor_id, pack_name).map_err(|e| e.to_json())
}

#[tauri::command]
pub fn get_processor_content_v1(
    request: rules_api::GetRuleContentRequest,
) -> Result<rules_api::GetRuleContentResponse, String> {
    rules_api::validate_get_rule_content_request(&request)?;
    let content = get_processor_content(
        request.kind.legacy_processor_mode().to_string(),
        request.pack_name,
    )?;
    Ok(rules_api::get_rule_content_response(content))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorPackTransfer {
    pack_name: String,
    enable: bool,
}

#[tauri::command]
pub fn get_processor_packs() -> Vec<ProcessorPackTransfer> {
    let packs = read_processors_from_appdir();

    let mut ret: Vec<ProcessorPackTransfer> = Vec::new();

    for ref pack in packs {
        ret.push(ProcessorPackTransfer {
            pack_name: pack.pack_name.to_string(),
            enable: pack.is_enable(),
        })
    }

    ret
}

#[tauri::command]
pub fn get_processor_packs_v1(
    request: rules_api::ListRulePacksRequest,
) -> Result<rules_api::ListRulePacksResponse, String> {
    rules_api::validate_list_rule_packs_request(&request)?;
    let packs = get_processor_packs()
        .into_iter()
        .map(|pack| rules_api::RulePackSummary {
            pack_name: pack.pack_name,
            enabled: pack.enable,
        })
        .collect::<Vec<_>>();

    Ok(rules_api::list_rule_packs_response(packs))
}

#[tauri::command]
pub fn validate_typed_rules_v1(
    request: rules_api::SaveTypedRulesRequest,
) -> Result<rules_api::ValidateTypedRulesResponse, String> {
    rules_api::validate_save_typed_rules_request(&request)?;
    Ok(rules_api::validate_typed_rules_response(&request.rules))
}

#[tauri::command]
pub fn save_rule_pack_rules_v1(
    request: rules_api::SaveRulePackRulesRequest,
) -> Result<rules_api::RulePackMutationResponse, String> {
    rules_api::parse_save_rule_pack_rules_request(&request)?;
    write_rule_pack_rules(request.pack_name.as_str(), request.content.as_str())
        .map_err(|err| format!("save rule pack rules failed: {err}"))?;
    remove_legacy_processor_files(request.pack_name.as_str())
        .map_err(|err| format!("remove legacy processor files failed: {err}"))?;
    write_processor_pack_status(request.pack_name.as_str(), request.enabled)
        .map_err(|err| format!("write pack status failed: {err}"))?;

    Ok(rules_api::rule_pack_mutation_response(true))
}

#[tauri::command]
pub fn get_rule_pack_rules_v1(
    request: rules_api::GetRulePackRulesRequest,
) -> Result<rules_api::RulePackRulesResponse, String> {
    rules_api::validate_get_rule_pack_rules_request(&request)?;
    let content = match read_rule_pack_rules(request.pack_name.as_str()) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(format!("read rule pack rules failed: {err}")),
    };

    rules_api::rule_pack_rules_response(
        request.pack_name.clone(),
        rule_pack_enabled(request.pack_name.as_str()),
        content,
    )
}

#[tauri::command]
pub async fn add_processor_pack(
    state: State<'_, ProxyState>,
    pack_name: String,
    enable: bool,
) -> Result<(), String> {
    let ret = create_pack_dir(pack_name.as_str());

    if let Err(err) = ret {
        return Err(format!("create pack dir failed: {err}"));
    }

    let mut state = state.lock().await;

    if state.is_some() {
        let msg = ProcessorChannelMessage::AddPack(pack_name, enable);

        match state.as_mut() {
            Some((_, sender, _, _, _, _)) => {
                if let Err(err) = sender.send(msg).await {
                    return Err(format!("mpsc send message failed: {err}"));
                }
            }
            None => return Err("get mpsc message sender failed".to_string()),
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn add_processor_pack_v1(
    state: State<'_, ProxyState>,
    request: rules_api::AddRulePackRequest,
) -> Result<rules_api::RulePackMutationResponse, String> {
    rules_api::validate_add_rule_pack_request(&request)?;
    add_processor_pack(state, request.pack_name, request.enabled).await?;
    Ok(rules_api::rule_pack_mutation_response(true))
}

#[tauri::command]
pub async fn remove_processor_pack(
    state: State<'_, ProxyState>,
    pack_name: String,
) -> Result<(), String> {
    let ret = delete_pack_dir(pack_name.as_str());

    if let Err(err) = ret {
        return Err(format!("remove pack dir failed: {err}"));
    }
    remove_processor_pack_status(pack_name.as_str())
        .map_err(|err| format!("remove pack status failed: {err}"))?;

    let mut state = state.lock().await;

    if state.is_some() {
        let msg = ProcessorChannelMessage::RemovePack(pack_name);

        match state.as_mut() {
            Some((_, sender, _, _, _, _)) => {
                if let Err(err) = sender.send(msg).await {
                    return Err(format!("mpsc send message failed: {err}"));
                }
            }
            None => return Err("get mpsc message sender failed".to_string()),
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn remove_processor_pack_v1(
    state: State<'_, ProxyState>,
    request: rules_api::RemoveRulePackRequest,
) -> Result<rules_api::RulePackMutationResponse, String> {
    rules_api::validate_remove_rule_pack_request(&request)?;
    remove_processor_pack(state, request.pack_name).await?;
    Ok(rules_api::rule_pack_mutation_response(true))
}

#[tauri::command]
pub async fn update_processor_pack_status(
    state: State<'_, ProxyState>,
    pack_name: String,
    status: bool,
) -> Result<(), String> {
    let ret = write_processor_pack_status(pack_name.as_str(), status);

    if let Err(err) = ret {
        return Err(format!("write pack status failed: {err}"));
    }

    let mut state = state.lock().await;

    if state.is_some() {
        let msg = ProcessorChannelMessage::UpdatePackStatus(pack_name, status);

        match state.as_mut() {
            Some((_, sender, _, _, _, _)) => {
                if let Err(err) = sender.send(msg).await {
                    return Err(format!("mpsc send message failed: {err}"));
                }
            }
            None => return Err("get mpsc message sender failed".to_string()),
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn update_processor_pack_status_v1(
    state: State<'_, ProxyState>,
    request: rules_api::UpdateRulePackStatusRequest,
) -> Result<rules_api::RulePackMutationResponse, String> {
    rules_api::validate_update_rule_pack_status_request(&request)?;
    update_processor_pack_status(state, request.pack_name, request.enabled).await?;
    Ok(rules_api::rule_pack_mutation_response(true))
}
