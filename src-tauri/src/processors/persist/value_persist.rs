use crate::app_conf::app_value_dir;
use std::{fs, io::Write, path::PathBuf};

pub fn read_value_list_from_appdir() -> Result<Vec<String>, std::io::Error> {
    let files = fs::read_dir(app_value_dir());

    files
        .map(|d| d.flatten().map(|f| f.file_name().into_string().unwrap()))
        .map(|c| c.collect())
}

pub fn read_value<S: AsRef<str>>(name: S) -> std::io::Result<String> {
    let path = app_value_dir().join(name.as_ref());

    fs::read_to_string(path)
}

pub fn write_value<S: AsRef<str>, V: AsRef<str>>(name: S, value: V) -> std::io::Result<()> {
    let mut file = ensure_value_file(name)?;
    file.write_all(value.as_ref().as_bytes())?;
    Ok(())
}

pub fn delete_value<S: AsRef<str>>(name: S) -> std::io::Result<()> {
    let path = app_value_dir().join(name.as_ref());

    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(path)
}

fn ensure_value_dir() -> Result<PathBuf, std::io::Error> {
    let value_dir = app_value_dir();

    super::ensure_dir(&value_dir)?;

    Ok(value_dir)
}

fn ensure_value_file<S: AsRef<str>>(name: S) -> Result<fs::File, std::io::Error> {
    let dir = ensure_value_dir()?;

    let file = dir.join(name.as_ref());

    fs::File::create(file)
}
