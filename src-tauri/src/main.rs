// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sys_events::handle_sys_events;

mod app_conf;
mod ca;
mod commands;
mod error;
mod events;
mod processors;
mod proxy;
mod sys_events;
mod window;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

fn main() {
    let context = tauri::generate_context!();

    let app = tauri::Builder::default()
        .plugin(proxy::init())
        .menu(window::initial_menu(tauri::Menu::os_default(&context.package_info().name)))
        .on_menu_event(window::register_menu_events)
        .setup(|_app| {
            if let Err(e) = app_conf::init() {
                return Err(Box::new(e));
            }

            simplelog::CombinedLogger::init(vec![simplelog::WriteLogger::new(
                #[cfg(debug_assertions)]
                    simplelog::LevelFilter::Trace,
                #[cfg(not(debug_assertions))]
                    simplelog::LevelFilter::Debug,
                simplelog::ConfigBuilder::new()
                    .add_filter_allow_str("proxyman")
                    .build(),
                std::fs::File::create(app_conf::app_logger_file()).unwrap(),
            )])
                .unwrap();

            Ok(())
        })
        .build(context)
        .expect("error while running tauri application");

    app.run(handle_sys_events);
}
