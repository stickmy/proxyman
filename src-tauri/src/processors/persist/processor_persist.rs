use std::collections::HashMap;
use std::{fs, io::Write};

use snafu::ResultExt;

use crate::app_conf;
use crate::error::processor_error::ReadStatusError;
use crate::error::ProcessorStatusError;
use crate::error::{
    self,
    processor_error::{ProcessorErrorKind, ReadError},
    Error, ProcessorError,
};
use crate::processors::http_processor::delay::RequestDelayProcessor;
use crate::processors::http_processor::redirect::RequestRedirectProcessor;
use crate::processors::http_processor::response::ResponseProcessor;
use crate::processors::processor_id::ProcessorID;
use crate::processors::processor_pack::ProcessorPack;

pub type ProcessorPackStatus = HashMap<String, bool>;

pub fn read_processor_packs_status() -> Result<ProcessorPackStatus, Error> {
    let str = fs::read_to_string(app_conf::app_processor_pack_status_file())
        .context(ReadStatusError {})
        .context(ProcessorStatusError {})?;

    serde_json::from_str(str.as_str()).map_err(|err| Error::ProcessorStatus {
        source: ProcessorErrorKind::Fmt {},
    })
}

pub fn write_processor_pack_status(pack_name: &str, enable: bool) -> Result<(), Error> {
    let mut status = read_processor_packs_status().unwrap_or_default();

    status.insert(pack_name.to_string(), enable);

    let str = serde_json::to_string::<ProcessorPackStatus>(&status).map_err(|err| {
        Error::ProcessorPack {
            source: ProcessorErrorKind::Fmt {},
        }
    })?;

    fs::write(app_conf::app_processor_pack_status_file(), str).map_err(|err| Error::ProcessorPack {
        source: ProcessorErrorKind::Write { source: err },
    })
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
                                        let content = fs::read_to_string(&file.path());

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

pub fn create_pack_dir(pack_name: &str) -> std::io::Result<()> {
    super::ensure_dir(app_conf::app_rule_dir())?;
    let dir = app_conf::app_rule_dir().join(pack_name);
    super::ensure_dir(dir)
}

pub fn delete_pack_dir(pack_name: &str) -> std::io::Result<()> {
    let dir = app_conf::app_rule_dir().join(pack_name);

    if !dir.exists() {
        return Ok(());
    }

    fs::remove_dir_all(dir)
}

pub fn write_processor(id: ProcessorID, content: &str, pack_name: &str) -> std::io::Result<()> {
    let mut file = ensure_processor_file(pack_name, id)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn read_processor(id: ProcessorID, pack_name: String) -> Result<String, error::Error> {
    let file = app_conf::app_rule_dir()
        .join(pack_name)
        .join(id.to_string());

    let metadata = fs::metadata(&file)
        .context(ReadError {})
        .context(ProcessorError { id })?;

    if !metadata.is_file() {
        return Err(Error::Processor {
            id,
            source: ProcessorErrorKind::NotFound {},
        });
    }

    fs::read_to_string(file)
        .context(ReadError {})
        .context(ProcessorError { id })
}

fn ensure_processor_file(
    pack_name: &str,
    processor_id: ProcessorID,
) -> Result<fs::File, std::io::Error> {
    let app_rule_path = app_conf::app_rule_dir();
    super::ensure_dir(&app_rule_path)?;

    let processor_dir = app_rule_path.join(pack_name);
    super::ensure_dir(&processor_dir)?;

    let path = processor_dir.join(processor_id.to_string());

    fs::File::create(path)
}
