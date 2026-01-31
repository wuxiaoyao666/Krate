use crate::commands::archive::{create_archive, extract_archive};
use crate::commands::image::{get_image_info, resize_image};
use crate::commands::network::{kill_process, scan_ports};
use crate::commands::system::{get_system_info, SystemState};

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(SystemState::new()) // 系统信息
        .invoke_handler(tauri::generate_handler![
            resize_image,
            get_image_info,
            scan_ports,
            kill_process,
            create_archive,
            extract_archive,
            get_system_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
