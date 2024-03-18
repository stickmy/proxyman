use crate::app_conf;

#[tauri::command]
pub async fn set_app_setting(setting: app_conf::AppSetting) -> () {
    log::trace!("set_app_setting, {:?}", setting);

    app_conf::save_app_setting(setting);
}

#[tauri::command]
pub async fn get_app_setting() -> app_conf::AppSetting {
    app_conf::get_app_setting()
}
