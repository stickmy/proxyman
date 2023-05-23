// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sys_events::handle_sys_events;
use tauri::{LogicalSize, Manager, Size};

mod app_conf;
mod ca;
mod commands;
mod error;
mod events;
mod processors;
mod proxy;
mod sys_events;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

fn main() {
    let app = tauri::Builder::default()
        .plugin(proxy::init())
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            window
                .set_size(Size::Logical(LogicalSize {
                    width: 1400.0,
                    height: 800.0,
                }))
                .unwrap();

            if let Err(e) = app_conf::init() {
                return Err(Box::new(e));
            }

            simplelog::CombinedLogger::init(vec![simplelog::WriteLogger::new(
                #[cfg(debug_assertions)]
                simplelog::LevelFilter::Trace,
                #[cfg(not(debug_assertions))]
                simplelog::LevelFilter::Error,
                simplelog::ConfigBuilder::new()
                    .add_filter_allow_str("proxyman")
                    .build(),
                std::fs::File::create(app_conf::app_logger_file()).unwrap(),
            )])
            .unwrap();

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(handle_sys_events);
}
