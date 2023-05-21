use std::{future::Future, sync::Arc};

use tauri::{async_runtime::Mutex, State};
use tokio::sync::mpsc::{self, Sender};

use crate::{
    processors::{
        http_processor::{
            delay::{RequestDelayProcessor, RequestDelayRule},
            redirect::RequestRedirectProcessor,
            response::{ResponseMapping, ResponseProcessor},
            HttpProcessor, ProcessorRuleParser,
        },
        processor_id::ProcessorID,
    },
    proxy::ProxyState,
};

use self::file_storage::FileStorage;

mod file_storage;

pub(crate) enum ProcessorChannelMessage {
    Redirect(Vec<[String; 2]>),
    Delay(RequestDelayRule),
    Response(ResponseMapping),
    RemoveResponse(String),
}

pub(crate) fn init() -> (
    Arc<Mutex<HttpProcessor>>,
    Sender<ProcessorChannelMessage>,
    impl Future<Output = ()>,
) {
    let redirect_processor = RequestRedirectProcessor::from_file();
    let delay_processor = RequestDelayProcessor::from_file();

    let processor = Arc::new(Mutex::new(
        HttpProcessor::builder()
            .set_redirect_processor(redirect_processor)
            .set_delay_processor(delay_processor)
            .set_response_processor(ResponseProcessor::default()),
    ));

    let (tx, mut rx) = mpsc::channel::<ProcessorChannelMessage>(1);

    let processor_setter = Arc::clone(&processor);
    let receiver = async move {
        log::debug!("Start listen ProcessorChannelMessage ...");

        while let Some(message) = rx.recv().await {
            let mut processor_setter = processor_setter.lock().await;

            match message {
                ProcessorChannelMessage::Redirect(mapping) => {
                    if let Some(ref mut redirect) = processor_setter.redirect {
                        redirect.set_redirects_mapping(mapping);
                    }
                }
                ProcessorChannelMessage::Delay(mappings) => {
                    if let Some(ref mut delay) = processor_setter.delay {
                        delay.set_delay_mapping(mappings);
                    }
                }
                ProcessorChannelMessage::Response(mapping) => {
                    if let Some(ref mut response) = processor_setter.response {
                        response.append_mapping(mapping);
                    }
                }
                ProcessorChannelMessage::RemoveResponse(req_pattern) => {
                    if let Some(ref mut response) = processor_setter.response {
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
    content: String,
) -> Result<bool, String> {
    log::debug!("Set Processor - mode: {}, content: {}", mode, content);

    let processor_id = ProcessorID::try_from(mode)?;

    let save_ret = match processor_id {
        ProcessorID::REDIRECT => RequestRedirectProcessor::save(content.as_str()),
        ProcessorID::DELAY => RequestDelayProcessor::save(content.as_str()),
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
                    RequestRedirectProcessor::parse_rule(content.as_str()),
                )),
                ProcessorID::DELAY => Some(ProcessorChannelMessage::Delay(
                    RequestDelayProcessor::parse_rule(content.as_str()),
                )),
                ProcessorID::RESPONSE => ResponseProcessor::parse_rule(content.as_str())
                    .map(ProcessorChannelMessage::Response),
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
    req: String,
) -> Result<(), String> {
    let mut state = state.lock().await;

    match state.as_mut() {
        Some((_, processor, _, _, _)) => {
            if let Err(e) = processor
                .send(ProcessorChannelMessage::RemoveResponse(req))
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
pub fn get_processor_content(mode: String) -> Result<String, String> {
    let processor_id = ProcessorID::try_from(mode)?;

    let ret = if processor_id == ProcessorID::REDIRECT {
        RequestRedirectProcessor::into_string()
    } else if processor_id == ProcessorID::DELAY {
        RequestDelayProcessor::into_string()
    } else {
        return Err("Unsupport rule".to_string());
    };

    ret.map_err(|e| e.to_string())
}
