use std::ffi::OsString;

use crate::processors::persist::value_persist::{
    delete_value, read_value, read_value_list_from_appdir, write_value,
};

#[tauri::command]
pub async fn get_value_list() -> Vec<OsString> {
    read_value_list_from_appdir().unwrap_or_default()
}

#[tauri::command]
pub async fn get_value_content(name: String) -> Result<String, String> {
    read_value(name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_value(name: String, value: String) -> Result<(), String> {
    write_value(name, value).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_value(name: String) -> Result<(), String> {
    delete_value(name).map_err(|e| e.to_string())
}
