mod clipboard;
mod commands;
mod storage;

use std::sync::Arc;
use tauri::Manager;
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use storage::database::Database;
use storage::images;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .plugin(tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build())
        .setup(|app| {
            log::info!("MeClipboard setup 开始");

            // 获取应用数据目录
            let data_dir = app
                .path()
                .app_local_data_dir()
                .expect("无法获取应用数据目录");

            // 初始化数据库
            let db = Arc::new(
                Database::open(data_dir).expect("无法打开数据库"),
            );
            app.manage(db.clone());

            // ===== 系统托盘 =====
            let window = app.get_webview_window("main").unwrap();
            let window_for_tray = window.clone();

            let show_item = MenuItemBuilder::with_id("show", "显示").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&quit_item)
                .build()?;

            // 托盘图标：如果默认图标不可用则跳过（不影响核心功能）
            if let Some(icon) = app.default_window_icon() {
                let _tray = TrayIconBuilder::new()
                    .icon(icon.clone())
                    .tooltip("MeClipboard - 剪贴板历史")
                    .menu(&menu)
                    .on_menu_event(|app_handle, event| {
                        match event.id().as_ref() {
                            "show" => {
                                if let Some(w) = app_handle.get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                            }
                            "quit" => {
                                app_handle.exit(0);
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(move |_tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let _ = window_for_tray.unminimize();
                            let _ = window_for_tray.show();
                            let _ = window_for_tray.set_focus();
                        }
                    })
                    .build(app)?;
            } else {
                log::warn!("默认窗口图标不可用，跳过系统托盘设置");
            }

            // ===== 全局快捷键 Alt+V =====
            let win_for_hotkey = window.clone();
            app.global_shortcut().on_shortcut("Alt+V", move |_app, _shortcut, _event| {
                let _ = win_for_hotkey.unminimize();
                let _ = win_for_hotkey.show();
                let _ = win_for_hotkey.set_focus();
            })?;

            // ===== 关闭 → 隐藏到托盘 =====
            let window_for_close = window.clone();
            window_for_close.clone().on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_for_close.hide();
                }
            });

            // ===== 窗口默认不可见 (visible: false)，只在非自启模式下显示 =====
            if let Ok(settings) = db.get_all_settings() {
                if !settings.autostart {
                    let _ = window.show();
                    log::info!("非自启模式：显示窗口");
                } else {
                    log::info!("自启模式：窗口保持在系统托盘");
                }
            } else {
                // 读取设置失败，默认显示窗口
                let _ = window.show();
            }

            // 启动剪贴板监听
            clipboard::monitor::start_monitor(app.handle().clone(), db.clone());

            // 启动时清理
            run_cleanup(&db);
            log::info!("启动时清理完成");

            // 每小时清理
            let cleanup_db = db.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
                run_cleanup(&cleanup_db);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_history,
            commands::search_clips,
            commands::toggle_pin,
            commands::delete_item,
            commands::restore_to_clipboard,
            commands::restore_deleted_item,
            commands::get_settings,
            commands::update_settings,
            commands::get_storage_stats,
            commands::get_image_data_url,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}

/// 执行保留天数清理 + 容量上限清理
fn run_cleanup(db: &Arc<Database>) {
    let data_dir = &db.data_dir;

    if let Ok(settings) = db.get_all_settings() {
        if settings.retention_days > 0 {
            match db.cleanup_old_clips(settings.retention_days) {
                Ok(image_paths) => {
                    for path in &image_paths {
                        images::delete_image_file(data_dir, path);
                    }
                    if !image_paths.is_empty() {
                        log::info!("过期清理: 删除了 {} 条记录", image_paths.len());
                    }
                }
                Err(e) => log::error!("过期清理失败: {}", e),
            }
        }

        let cap_bytes = settings.storage_cap_mb as u64 * 1024 * 1024;
        let current_size = db.get_storage_stats().unwrap_or((0, 0)).1
            + images::images_dir_size(data_dir);

        if current_size > cap_bytes {
            log::info!(
                "容量超限: {} MB (上限 {} MB), 开始清理",
                current_size / (1024 * 1024),
                settings.storage_cap_mb
            );
            for _ in 0..100 {
                let size_now = db.get_storage_stats().unwrap_or((0, 0)).1
                    + images::images_dir_size(data_dir);
                if size_now <= cap_bytes {
                    break;
                }
                match db.delete_oldest_unpinned(10) {
                    Ok(image_paths) => {
                        for path in &image_paths {
                            images::delete_image_file(data_dir, path);
                        }
                        if image_paths.is_empty() {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("容量清理失败: {}", e);
                        break;
                    }
                }
            }
        }
    }
}
