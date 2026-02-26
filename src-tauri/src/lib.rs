use crate::commands::archive::{create_archive, extract_archive};
use crate::commands::image::{get_image_info, resize_image};
use crate::commands::network::{kill_process, scan_ports};
use crate::commands::proxy::{proxy_get_status, proxy_start, proxy_stop, ProxyState};
use crate::commands::system::{get_system_info, SystemState};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // === 1. 创建托盘菜单 ===
            let quit_i = MenuItem::with_id(app, "quit", "退出 Krate", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;
            // === 2. 构建托盘图标 ===
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone()) // 使用默认的应用图标
                .menu(&menu)
                .show_menu_on_left_click(false) // 左键不显示菜单
                .on_menu_event(|app, event| match event.id.as_ref() {
                    // 处理菜单点击
                    "quit" => app.exit(0), // 退出软件
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    // 处理托盘图标本身的点击 左键切换显示/隐藏
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        // 拦截关闭事件
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // 只拦截主窗口 其它子窗口直接关闭
                if window.label() == "main" {
                    // 移除关闭事件
                    api.prevent_close();
                    // 隐藏窗口
                    let _ = window.hide();
                }
            }
        })
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .manage(SystemState::new()) // 系统信息
        .manage(ProxyState::new())
        .invoke_handler(tauri::generate_handler![
            resize_image,
            get_image_info,
            scan_ports,
            kill_process,
            create_archive,
            extract_archive,
            get_system_info,
            proxy_start,
            proxy_stop,
            proxy_get_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
