use crate::app_conf;
use crate::error::{
    processor_error::{ProcessorErrorKind, ReadError},
    Error, ProcessorError,
};
use crate::processors::{
    http_processor::{delay::RequestDelayProcessor, redirect::RequestRedirectProcessor},
    processor_id::ProcessorID,
    Processor,
};
use snafu::ResultExt;
use std::io::{self, Write};
use std::{fs, path};

pub trait FileStorage: From<String> + Default + Processor {
    fn from_file() -> Self;

    fn into_string() -> Result<String, Error>;

    fn save(content: &str) -> Result<(), io::Error>;
}

impl FileStorage for RequestRedirectProcessor {
    fn from_file() -> Self {
        let content = fs::read_to_string(get_file_path(ProcessorID::REDIRECT));

        match content {
            Ok(content) => content.into(),
            Err(_) => Self::default(),
        }
    }

    fn into_string() -> Result<String, Error> {
        get_content(ProcessorID::REDIRECT)
    }

    fn save(content: &str) -> Result<(), io::Error> {
        save_as_file(ProcessorID::REDIRECT, content)
    }
}

impl FileStorage for RequestDelayProcessor {
    fn from_file() -> Self {
        let content = fs::read_to_string(get_file_path(ProcessorID::DELAY));

        match content {
            Ok(content) => content.into(),
            Err(_) => Self::default(),
        }
    }

    fn into_string() -> Result<String, Error> {
        get_content(ProcessorID::DELAY)
    }

    fn save(content: &str) -> Result<(), io::Error> {
        save_as_file(ProcessorID::DELAY, content)
    }
}

fn get_file_path(id: ProcessorID) -> path::PathBuf {
    let app_rule = app_conf::app_rule_dir();
    return path::Path::new(&app_rule).join::<&str>(id.into());
}

fn save_as_file(id: ProcessorID, content: &str) -> Result<(), std::io::Error> {
    let mut f = ensure_file(id.into())?;
    f.write(content.as_bytes()).map(|_| ())
}

fn get_content(id: ProcessorID) -> Result<String, Error> {
    let file_path = get_file_path(id);

    let metadata = fs::metadata(&file_path)
        .context(ReadError {})
        .context(ProcessorError { id })?;

    if !metadata.is_file() {
        return Err(Error::Processor {
            id,
            source: ProcessorErrorKind::NotFound {},
        });
    }

    fs::read_to_string(file_path)
        .context(ReadError {})
        .context(ProcessorError { id })
}

fn ensure_file(file_name: &str) -> Result<fs::File, std::io::Error> {
    match fs::metadata(app_conf::app_rule_dir()) {
        Ok(meta) => {
            if !meta.is_dir() {
                fs::create_dir(app_conf::app_rule_dir())?;
            }
        }
        Err(_) => {
            fs::create_dir(app_conf::app_rule_dir())?;
        }
    }

    let path = path::Path::new(&app_conf::app_rule_dir()).join(file_name);

    fs::File::create(path)
}
