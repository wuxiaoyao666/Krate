use crate::commands::image::{crop_image, get_image_info, resize_image};
use crate::commands::network::{kill_process, scan_ports};

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            resize_image,
            get_image_info,
            crop_image,
            scan_ports,
            kill_process
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
