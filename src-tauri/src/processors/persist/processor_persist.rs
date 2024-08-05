use std::collections::HashMap;
use std::{fs, io::Write};

use super::error::{ProcessorError, ProcessorPackError};
use crate::app_conf;
use crate::processors::{
    http_processor::{
        delay::RequestDelayProcessor, redirect::RequestRedirectProcessor,
        response::ResponseProcessor,
    },
    processor_id::ProcessorID,
    processor_pack::ProcessorPack,
};

pub type ProcessorPackStatus = HashMap<String, bool>;

pub fn read_processor_packs_status() -> anyhow::Result<ProcessorPackStatus, ProcessorPackError> {
    let str = fs::read_to_string(app_conf::app_processor_pack_status_file())
        .map_err(ProcessorPackError::from)?;

    serde_json::from_str(str.as_str()).map_err(ProcessorPackError::from)
}

pub fn write_processor_pack_status(
    pack_name: &str,
    enable: bool,
) -> anyhow::Result<(), ProcessorPackError> {
    let mut status = read_processor_packs_status().unwrap_or_default();

    status.insert(pack_name.to_string(), enable);

    let str =
        serde_json::to_string::<ProcessorPackStatus>(&status).map_err(ProcessorPackError::from)?;

    fs::write(app_conf::app_processor_pack_status_file(), str).map_err(ProcessorPackError::from)
}

pub fn read_processors_from_appdir() -> Vec<ProcessorPack> {
    let pack_status = read_processor_packs_status();

    // TODO: refactor with Result<Vec<Interceptor>, std::io::Error> inner fn.
    let mut packs = Vec::<ProcessorPack>::new();

    let children = fs::read_dir(app_conf::app_rule_dir());

    if let Ok(children) = children {
        for dir in children.flatten() {
            let metadata = dir.metadata();
            if let Ok(metadata) = metadata {
                if metadata.is_dir() {
                    if let Ok(dir_name) = dir.file_name().into_string() {
                        let mut pack = ProcessorPack::new(dir_name, false);

                        // initialize pack enable status
                        if let Ok(ref status) = pack_status {
                            if status.get(&pack.pack_name) == Some(&true) {
                                pack.enable();
                            }
                        }

                        let files = fs::read_dir(dir.path());
                        if let Ok(files) = files {
                            for file in files.flatten() {
                                if let Ok(file_name) = file.file_name().into_string() {
                                    let processor_id = ProcessorID::try_from(file_name);
                                    if let Ok(processor_id) = processor_id {
                                        let content = fs::read_to_string(file.path());

                                        if let Ok(content) = content {
                                            match processor_id {
                                                ProcessorID::REDIRECT => pack.set_redirect(
                                                    RequestRedirectProcessor::from(content),
                                                ),
                                                ProcessorID::DELAY => pack.set_delay(
                                                    RequestDelayProcessor::from(content),
                                                ),
                                                ProcessorID::RESPONSE => pack
                                                    .set_response(ResponseProcessor::from(content)),
                                                _ => {
                                                    log::debug!(
                                                        "processor file: {processor_id} is ignored"
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        packs.push(pack);
                    }
                }
            }
        }
    }

    packs
}

pub fn create_pack_dir(pack_name: &str) -> anyhow::Result<(), ProcessorPackError> {
    super::ensure_dir(app_conf::app_rule_dir()).map_err(ProcessorPackError::from)?;
    let dir = app_conf::app_rule_dir().join(pack_name);
    super::ensure_dir(dir).map_err(ProcessorPackError::from)
}

pub fn delete_pack_dir(pack_name: &str) -> anyhow::Result<(), ProcessorPackError> {
    let dir = app_conf::app_rule_dir().join(pack_name);

    if !dir.exists() {
        return Ok(());
    }

    fs::remove_dir_all(dir).map_err(ProcessorPackError::from)
}

pub fn write_processor(
    id: ProcessorID,
    content: &str,
    pack_name: &str,
) -> anyhow::Result<(), ProcessorError> {
    let mut file = ensure_processor_file(pack_name, id).map_err(ProcessorError::from)?;
    file.write_all(content.as_bytes())
        .map_err(ProcessorError::from)?;
    Ok(())
}

pub fn read_processor(
    id: ProcessorID,
    pack_name: String,
) -> anyhow::Result<String, ProcessorError> {
    let file = app_conf::app_rule_dir()
        .join(pack_name)
        .join(id.to_string());

    let metadata = fs::metadata(&file).map_err(ProcessorError::from)?;

    if !metadata.is_file() {
        return Err(ProcessorError::NotFound(id.to_string()));
    }

    fs::read_to_string(file).map_err(ProcessorError::from)
}

fn ensure_processor_file(
    pack_name: &str,
    processor_id: ProcessorID,
) -> anyhow::Result<fs::File, ProcessorError> {
    let app_rule_path = app_conf::app_rule_dir();
    super::ensure_dir(&app_rule_path).map_err(ProcessorError::from)?;

    let processor_dir = app_rule_path.join(pack_name);
    super::ensure_dir(&processor_dir)?;

    let path = processor_dir.join(processor_id.to_string());

    fs::File::create(path).map_err(ProcessorError::from)
}
