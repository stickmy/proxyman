use std::collections::HashMap;
use std::{fs, io::Write, path::Path};

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
use crate::processors::http_processor::request_header::RequestHeaderProcessor;
use crate::processors::http_processor::response::ResponseProcessor;
use crate::processors::http_processor::response_header::ResponseHeaderProcessor;
use crate::processors::http_processor::typed_rules::TypedRuleProcessor;
use crate::processors::processor_id::ProcessorID;
use crate::processors::processor_pack::ProcessorPack;

pub type ProcessorPackStatus = HashMap<String, bool>;
pub const RULE_PACK_RULES_FILE: &str = "rules.rules";

pub fn read_processor_packs_status() -> Result<ProcessorPackStatus, Error> {
    let str = fs::read_to_string(app_conf::app_processor_pack_status_file())
        .context(ReadStatusError {})
        .context(ProcessorStatusError {})?;

    serde_json::from_str(str.as_str()).map_err(|_err| Error::ProcessorStatus {
        source: ProcessorErrorKind::Fmt {},
    })
}

pub fn write_processor_pack_status(pack_name: &str, enable: bool) -> Result<(), Error> {
    let mut status = read_processor_packs_status().unwrap_or_default();

    status.insert(pack_name.to_string(), enable);
    write_processor_packs_status(&status)
}

pub fn remove_processor_pack_status(pack_name: &str) -> Result<(), Error> {
    let mut status = read_processor_packs_status().unwrap_or_default();

    status.remove(pack_name);
    write_processor_packs_status(&status)
}

fn write_processor_packs_status(status: &ProcessorPackStatus) -> Result<(), Error> {
    let str = serde_json::to_string::<ProcessorPackStatus>(&status).map_err(|_err| {
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
    let pack_status = pack_status.unwrap_or_default();
    read_processors_from_rule_dir(app_conf::app_rule_dir().as_path(), &pack_status)
}

fn read_processors_from_rule_dir<P: AsRef<Path>>(
    rule_dir: P,
    pack_status: &ProcessorPackStatus,
) -> Vec<ProcessorPack> {
    // TODO: refactor with Result<Vec<Interceptor>, std::io::Error> inner fn.
    let mut packs = Vec::<ProcessorPack>::new();

    let children = fs::read_dir(rule_dir.as_ref());

    if let Ok(children) = children {
        for dir in children.flatten() {
            let metadata = dir.metadata();
            if let Ok(metadata) = metadata {
                if metadata.is_dir() {
                    if let Ok(dir_name) = dir.file_name().into_string() {
                        let mut pack = ProcessorPack::new(dir_name, false);

                        // initialize pack enable status
                        if pack_status.get(&pack.pack_name) == Some(&true) {
                            pack.enable();
                        }

                        let files = fs::read_dir(dir.path());
                        if let Ok(files) = files {
                            for file in files.flatten() {
                                if let Ok(file_name) = file.file_name().into_string() {
                                    if file_name == RULE_PACK_RULES_FILE {
                                        if let Ok(content) = fs::read_to_string(file.path()) {
                                            match crate::core_api::rules::parse_typed_rules_dsl(
                                                content.as_str(),
                                            )
                                            .and_then(TypedRuleProcessor::from_rules)
                                            {
                                                Ok(processor) => pack.set_typed_rules(processor),
                                                Err(err) => log::debug!(
                                                    "typed rule pack({}) is ignored: {err}",
                                                    pack.pack_name
                                                ),
                                            }
                                        }
                                        continue;
                                    }

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
                                                ProcessorID::REQUEST_HEADER => pack
                                                    .set_request_header(
                                                        RequestHeaderProcessor::from(content),
                                                    ),
                                                ProcessorID::RESPONSE_HEADER => pack
                                                    .set_response_header(
                                                        ResponseHeaderProcessor::from(content),
                                                    ),
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

pub fn write_rule_pack_rules(pack_name: &str, content: &str) -> std::io::Result<()> {
    write_rule_pack_rules_at(app_conf::app_rule_dir().as_path(), pack_name, content)
}

pub fn read_rule_pack_rules(pack_name: &str) -> std::io::Result<String> {
    read_rule_pack_rules_at(app_conf::app_rule_dir().as_path(), pack_name)
}

pub fn remove_legacy_processor_files(pack_name: &str) -> std::io::Result<()> {
    remove_legacy_processor_files_at(app_conf::app_rule_dir().as_path(), pack_name)
}

pub fn rule_pack_enabled(pack_name: &str) -> bool {
    read_processor_packs_status()
        .ok()
        .and_then(|status| status.get(pack_name).copied())
        .unwrap_or(false)
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

fn write_rule_pack_rules_at<P: AsRef<Path>>(
    rule_dir: P,
    pack_name: &str,
    content: &str,
) -> std::io::Result<()> {
    let rule_dir = rule_dir.as_ref();
    super::ensure_dir(rule_dir)?;
    let pack_dir = rule_dir.join(pack_name);
    super::ensure_dir(&pack_dir)?;
    fs::write(rule_pack_rules_path(rule_dir, pack_name), content)
}

fn read_rule_pack_rules_at<P: AsRef<Path>>(
    rule_dir: P,
    pack_name: &str,
) -> std::io::Result<String> {
    fs::read_to_string(rule_pack_rules_path(rule_dir.as_ref(), pack_name))
}

fn remove_legacy_processor_files_at<P: AsRef<Path>>(
    rule_dir: P,
    pack_name: &str,
) -> std::io::Result<()> {
    let pack_dir = rule_dir.as_ref().join(pack_name);
    for processor_id in legacy_processor_ids() {
        let file = pack_dir.join(processor_id.to_string());
        if file.exists() {
            fs::remove_file(file)?;
        }
    }
    Ok(())
}

fn rule_pack_rules_path(rule_dir: &Path, pack_name: &str) -> std::path::PathBuf {
    rule_dir.join(pack_name).join(RULE_PACK_RULES_FILE)
}

fn legacy_processor_ids() -> [ProcessorID; 5] {
    [
        ProcessorID::REDIRECT,
        ProcessorID::DELAY,
        ProcessorID::RESPONSE,
        ProcessorID::REQUEST_HEADER,
        ProcessorID::RESPONSE_HEADER,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    use crate::processors::{
        http_processor::HttpProcessor,
        processor::HttpProcessor as _,
    };
    use hyper::{Body, Request};

    #[test]
    fn rules_pack_persistence_writes_user_authored_rules_file() {
        let rule_dir = unique_test_rule_dir("write-rules");
        let content = "# preserve this comment\nredirect GET https://api.example.com/* https://mock.local\n";

        write_rule_pack_rules_at(&rule_dir, "default", content)
            .expect("rules pack should write");

        assert_eq!(
            read_rule_pack_rules_at(&rule_dir, "default").expect("rules pack should read"),
            content
        );
        assert!(rule_dir
            .join("default")
            .join(RULE_PACK_RULES_FILE)
            .is_file());

        let _ = fs::remove_dir_all(rule_dir);
    }

    #[test]
    fn rules_pack_persistence_removes_legacy_processor_files_only() {
        let rule_dir = unique_test_rule_dir("remove-legacy");
        let pack_dir = rule_dir.join("default");
        fs::create_dir_all(&pack_dir).expect("pack dir should be created");
        fs::write(pack_dir.join(RULE_PACK_RULES_FILE), "delay * * 1ms\n")
            .expect("rules file should be written");
        fs::write(pack_dir.join("notes.txt"), "keep").expect("notes should be written");

        for processor_id in legacy_processor_ids() {
            fs::write(pack_dir.join(processor_id.to_string()), "legacy")
                .expect("legacy processor file should be written");
        }

        remove_legacy_processor_files_at(&rule_dir, "default")
            .expect("legacy processor files should be removed");

        assert!(pack_dir.join(RULE_PACK_RULES_FILE).is_file());
        assert!(pack_dir.join("notes.txt").is_file());
        for processor_id in legacy_processor_ids() {
            assert!(!pack_dir.join(processor_id.to_string()).exists());
        }

        let _ = fs::remove_dir_all(rule_dir);
    }

    #[tokio::test]
    async fn rules_pack_persistence_loads_rules_file_into_runtime_processor() {
        let rule_dir = unique_test_rule_dir("load-rules");
        let pack_dir = rule_dir.join("default");
        fs::create_dir_all(&pack_dir).expect("pack dir should be created");
        fs::write(
            pack_dir.join(RULE_PACK_RULES_FILE),
            "request-header GET https://api.example.com/* x-runtime yes\n",
        )
        .expect("rules file should be written");
        let mut status = ProcessorPackStatus::new();
        status.insert("default".to_string(), true);

        let processor = HttpProcessor::new(read_processors_from_rule_dir(&rule_dir, &status));
        let req = Request::builder()
            .method("GET")
            .uri("https://api.example.com/users")
            .body(Body::empty())
            .expect("request should build");

        let req_or_res = processor.process_request(req).await;

        assert_eq!(req_or_res.req.headers().get("x-runtime").unwrap(), "yes");

        let _ = fs::remove_dir_all(rule_dir);
    }

    fn unique_test_rule_dir(name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("proxyman-{name}-{now}"))
    }
}
