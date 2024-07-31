fn main() {
    // tauri_build::build();
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .codegen(tauri_build::CodegenContext::new())
            .plugin(
                "proxy",
                tauri_build::InlinedPlugin::new().commands(&[
                    "start_proxy",
                    "stop_proxy",
                    "proxy_status",
                    "check_cert_installed",
                    "install_cert",
                    "get_value_list",
                    "get_value_content",
                    "save_value",
                    "remove_value",
                    "set_processor",
                    "get_processor_content",
                    "get_processor_packs",
                    "add_processor_pack",
                    "remove_processor_pack",
                    "update_processor_pack_status",
                    "turn_on_global_proxy",
                    "turn_off_global_proxy",
                    "set_app_setting",
                    "get_app_setting",
                ]),
            )
            .app_manifest(tauri_build::AppManifest::new().commands(&[])),
    )
    .expect("failed to run tauri-build");
}
