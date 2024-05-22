use std::process::Command;

use tauri::{CustomMenuItem, Menu, Submenu, WindowMenuEvent};

use crate::app_conf::{self};

pub fn initial_menu(menu: Menu) -> Menu {
    menu.add_submenu(Submenu::new(
        "Help",
        Menu::new().add_item(CustomMenuItem::new("openlog", "Open Logs")),
    ))
}

pub fn register_menu_events(event: WindowMenuEvent) {
    if let "openlog" = event.menu_item_id() {
        Command::new("open")
            .arg(app_conf::app_dir().as_os_str())
            .output()
            .unwrap();
    };
}
