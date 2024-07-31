use std::process::Command;

use tauri::{
    menu::{Menu, MenuItemBuilder, HELP_SUBMENU_ID},
    Wry,
};

use crate::app_conf::{self};

pub fn build_menu(app: &tauri::App) -> tauri::Result<Menu<Wry>> {
    let open_log = MenuItemBuilder::with_id("openlog", "Open Logs").build(app)?;

    let menu = Menu::default(app.handle())?;
    for item in menu.items()? {
        if let Some(ref mut submenu) = item.as_submenu() {
            if submenu.id() == HELP_SUBMENU_ID {
                submenu.append_items(&[&open_log])?;
            }
        }
    }

    app.on_menu_event(move |_, event| {
        if event.id == open_log.id() {
            Command::new("open")
                .arg(app_conf::app_dir().as_os_str())
                .output()
                .unwrap();
        }
    });

    Ok(menu)
}
