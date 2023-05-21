use std::io::{self, Write};
use std::{fs, path};

use crate::app_conf;
use crate::processors::{
    http_processor::{delay::RequestDelayProcessor, redirect::RequestRedirectProcessor},
    processor_id::ProcessorID,
    Processor,
};

pub trait FileStorage: From<String> + Default + Processor {
    fn from_file() -> Self;

    fn into_string() -> Result<String, io::Error>;

    fn save(content: &str) -> Result<(), io::Error>;
}

impl FileStorage for RequestRedirectProcessor {
    fn from_file() -> Self {
        let content = fs::read_to_string(get_file_path(ProcessorID::REDIRECT.into()));

        match content {
            Ok(content) => content.into(),
            Err(_) => Self::default(),
        }
    }

    fn into_string() -> Result<String, io::Error> {
        get_content(ProcessorID::REDIRECT.into())
    }

    fn save(content: &str) -> Result<(), io::Error> {
        save_as_file(ProcessorID::REDIRECT.into(), content)
    }
}

impl FileStorage for RequestDelayProcessor {
    fn from_file() -> Self {
        let content = fs::read_to_string(get_file_path(ProcessorID::DELAY.into()));

        match content {
            Ok(content) => content.into(),
            Err(_) => Self::default(),
        }
    }

    fn into_string() -> Result<String, io::Error> {
        get_content(ProcessorID::DELAY.into())
    }

    fn save(content: &str) -> Result<(), io::Error> {
        save_as_file(ProcessorID::DELAY.into(), content)
    }
}

fn get_file_path(mode: &str) -> path::PathBuf {
    let app_rule = app_conf::app_rule_dir();
    return path::Path::new(&app_rule).join(mode);
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

fn save_as_file(mode: &str, content: &str) -> Result<(), std::io::Error> {
    let mut f = ensure_file(mode)?;
    f.write(content.as_bytes()).map(|_| ())
}

fn get_content(mode: &str) -> Result<String, io::Error> {
    let file_path = get_file_path(mode);

    match fs::metadata(&file_path) {
        Ok(meta) => {
            if !meta.is_file() {
                return Err(io::ErrorKind::NotFound.into());
            }

            fs::read_to_string(file_path)
        }
        Err(e) => Err(e),
    }
}
