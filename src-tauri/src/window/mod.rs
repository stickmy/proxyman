use std::process::Command;

use tauri::{CustomMenuItem, LogicalSize, Menu, Size, Submenu, Window, WindowMenuEvent};

use crate::app_conf::{self};

pub fn initial_window(win: Window) -> () {
    win.set_size(Size::Logical(LogicalSize {
        width: 1200.0,
        height: 800.0,
    }))
    .unwrap();

    win.center().unwrap();
}

pub fn initial_menu() -> Menu {
    Menu::new()
        .add_submenu(Submenu::new("proxyman", Menu::new()))
        .add_submenu(Submenu::new(
            "帮助",
            Menu::new().add_item(CustomMenuItem::new("openlog", "打开日志文件")),
        ))
}

pub fn register_menu_events(event: WindowMenuEvent) {
    match event.menu_item_id() {
        "openlog" => {
            Command::new("open").arg(app_conf::app_dir().as_os_str()).output().unwrap();
        },
        _ => {}
    };
}
