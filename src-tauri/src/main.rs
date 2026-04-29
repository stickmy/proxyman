// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_conf;
mod ca;
mod commands;
mod core_api;
mod error;
mod events;
mod processors;
mod proxy;
mod storage;
mod sys_events;
mod window;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

fn main() {
    let context = tauri::generate_context!();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            if let Err(e) = app_conf::init() {
                return Err(Box::new(e));
            }

            proxy::set_proxy_state(app);

            let menu = window::build_menu(app)?;
            app.set_menu(menu)?;

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
        .invoke_handler(tauri::generate_handler![
            proxy::start_proxy,
            proxy::start_proxy_v1,
            proxy::stop_proxy,
            proxy::stop_proxy_v1,
            proxy::proxy_status,
            proxy::proxy_status_v1,
            commands::ca::check_cert_installed,
            commands::ca::check_cert_installed_v1,
            commands::ca::install_cert,
            commands::ca::install_cert_v1,
            commands::values::get_value_list,
            commands::values::get_value_content,
            commands::values::save_value,
            commands::values::remove_value,
            commands::processor::set_processor,
            commands::processor::set_processor_v1,
            commands::processor::get_processor_content,
            commands::processor::get_processor_content_v1,
            commands::processor::get_processor_packs,
            commands::processor::get_processor_packs_v1,
            commands::processor::validate_typed_rules_v1,
            commands::processor::save_rule_pack_rules_v1,
            commands::processor::get_rule_pack_rules_v1,
            commands::processor::add_processor_pack,
            commands::processor::add_processor_pack_v1,
            commands::processor::remove_processor_pack,
            commands::processor::remove_processor_pack_v1,
            commands::processor::update_processor_pack_status,
            commands::processor::update_processor_pack_status_v1,
            commands::session::search_session_events,
            commands::session::search_session_exchanges,
            commands::session::export_session_har,
            commands::session::import_session_har,
            commands::session::load_session_body,
            commands::session::replay_session_request,
            commands::global_proxy::turn_on_global_proxy,
            commands::global_proxy::turn_on_global_proxy_v1,
            commands::global_proxy::turn_off_global_proxy,
            commands::global_proxy::turn_off_global_proxy_v1,
            commands::global_proxy::system_proxy_status_v1,
            commands::app_setting::set_app_setting,
            commands::app_setting::get_app_setting,
        ])
        .build(context)
        .expect("error while running tauri application");

    app.run(sys_events::handle_sys_events);
}
