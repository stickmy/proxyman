use std::{future::Future, sync::Arc};

use serde::Serialize;
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};

use crate::core_api::rules as rules_api;
use crate::processors::persist::processor_persist::{
    read_rule_pack_rules, remove_legacy_processor_files, rule_pack_enabled,
    write_processor_pack_status, write_rule_pack_rules,
};
use crate::processors::processor_pack::ProcessorPack;
use crate::processors::{
    http_processor::{delay::RequestDelayRule, HttpProcessor},
    persist::processor_persist::read_processors_from_appdir,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorPackTransfer {
    pack_name: String,
    enable: bool,
}

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

pub fn validate_typed_rules_v1(
    request: rules_api::SaveTypedRulesRequest,
) -> Result<rules_api::ValidateTypedRulesResponse, String> {
    rules_api::validate_save_typed_rules_request(&request)?;
    Ok(rules_api::validate_typed_rules_response(&request.rules))
}

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
