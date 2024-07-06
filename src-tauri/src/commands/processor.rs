use std::{future::Future, sync::Arc};

use tauri::{async_runtime::Mutex, State};
use tokio::sync::mpsc::{self, Sender};

use crate::processors::parser::ProcessorRuleParser;
use crate::{
    error::{processor_error::ProcessorErrorKind, Error},
    processors::{
        http_processor::{
            delay::{RequestDelayProcessor, RequestDelayRule},
            redirect::RequestRedirectProcessor,
            response::{ResponseMapping, ResponseProcessor},
            HttpProcessor,
        },
        persist::{read_interceptors_from_appdir, read_processor, write_processor},
        processor_id::ProcessorID,
    },
    proxy::ProxyState,
};

pub(crate) enum ProcessorChannelMessage {
    Redirect(String, Vec<[String; 2]>),
    Delay(String, RequestDelayRule),
    Response(String, ResponseMapping),
    RemoveResponse(String, String),
}

pub(crate) fn init() -> (
    Arc<Mutex<HttpProcessor>>,
    Sender<ProcessorChannelMessage>,
    impl Future<Output = ()>,
) {
    let packs = read_interceptors_from_appdir();

    let processor = Arc::new(Mutex::new(HttpProcessor::new(packs)));

    let (tx, mut rx) = mpsc::channel::<ProcessorChannelMessage>(1);

    let processor_setter = Arc::clone(&processor);
    let receiver = async move {
        log::debug!("Start listening ProcessorChannelMessage ...");

        while let Some(message) = rx.recv().await {
            let mut processor_setter = processor_setter.lock().await;

            match message {
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
                        response.append_mapping(mapping);
                    }
                }
                ProcessorChannelMessage::RemoveResponse(pack_name, req_pattern) => {
                    if let Some(response) = processor_setter.get_response_mut(pack_name) {
                        response.remove_mapping(req_pattern);
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

    let save_ret = match processor_id {
        ProcessorID::REDIRECT | ProcessorID::DELAY => {
            write_processor(processor_id, content.as_str(), pack_name.as_str())
        }
        ProcessorID::RESPONSE => Ok(()),
        _ => return Err("Unsupport processor".into()),
    };

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
                ProcessorID::RESPONSE => ResponseProcessor::parse_rule(content.as_str())
                    .map(|x| ProcessorChannelMessage::Response(pack_name, x)),
                _ => return Err("Unsupport processor".into()),
            }
        };

        match msg {
            Some(msg) => match state.as_mut() {
                Some((_, processor, _, _, _)) => {
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
pub(crate) async fn remove_response_mapping(
    state: State<'_, ProxyState>,
    pack_name: String,
    req: String,
) -> Result<(), String> {
    let mut state = state.lock().await;

    match state.as_mut() {
        Some((_, processor, _, _, _)) => {
            if let Err(e) = processor
                .send(ProcessorChannelMessage::RemoveResponse(pack_name, req))
                .await
            {
                log::error!("Remove ResponseProcessor mapping, {e}");
                return Err(format!("Remove mapping failed: {e}"));
            }
        }
        None => return Err("get proxy state failed".to_string()),
    }

    Ok(())
}

#[tauri::command]
pub fn get_processor_content(mode: String, pack_name: String) -> Result<String, String> {
    let processor_id = ProcessorID::try_from(mode)?;

    let ret = match processor_id {
        ProcessorID::REDIRECT | ProcessorID::DELAY => read_processor(processor_id, pack_name),
        _ => {
            return Err(Error::Processor {
                id: processor_id,
                source: ProcessorErrorKind::Unsupport {},
            }
            .to_json())
        }
    };

    ret.map_err(|e| e.to_json())
}
