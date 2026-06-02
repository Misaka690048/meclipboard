use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;
use sha2::{Sha256, Digest};

use crate::clipboard::models::ClipItem;
use crate::storage::database::Database;
use crate::storage::images;

/// 剪贴板监听器状态
struct MonitorState {
    last_text: Option<String>,       // 用于去重比较（trim 后）
    last_image_hash: Option<String>,
}

/// 启动剪贴板轮询（在独立线程中运行，带 panic 恢复）
pub fn start_monitor(app_handle: AppHandle, db: Arc<Database>) {
    std::thread::spawn(move || {
        log::info!("剪贴板监听线程已启动");

        let mut state = MonitorState {
            last_text: None,
            last_image_hash: None,
        };

        loop {
            std::thread::sleep(Duration::from_millis(500));

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                check_clipboard(&app_handle, &db, &mut state);
            }));
            if let Err(e) = result {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                log::error!("剪贴板监听 panic 已恢复: {}", msg);
                state.last_text = None;
                state.last_image_hash = None;
            }
        }
    });
}

fn check_clipboard(app_handle: &AppHandle, db: &Database, state: &mut MonitorState) {
    let now = chrono::Utc::now().timestamp();

    // 检查文字
    match app_handle.clipboard().read_text() {
        Ok(text) => {
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() && state.last_text.as_deref() != Some(&trimmed) {
                let preview: String = text.chars().take(50).collect();
                log::info!("检测到新文字: {}...", preview);
                state.last_text = Some(trimmed);
                state.last_image_hash = None;

                // 存储原文（不去空格），trim 仅用于去重比较
                let item = ClipItem::new_text(0, text, now);
                match db.insert_clip(&item) {
                    Ok(new_id) => {
                        let mut item_with_id = item.clone();
                        item_with_id.id = new_id;
                        log::info!("文字已存储，id={}", new_id);
                        let _ = app_handle.emit("clipboard-changed", &item_with_id);
                    }
                    Err(e) => log::error!("存储文字条目失败: {}", e),
                }
            }
            return;
        }
        Err(ref e) => {
            log::debug!("read_text 无文字: {:?}", e);
        }
    }

    // 检查图片
    match app_handle.clipboard().read_image() {
        Ok(rgba_image) => {
            let rgba_bytes = rgba_image.rgba();
            let width = rgba_image.width();
            let height = rgba_image.height();

            // 拒绝空/退化图片
            if width == 0 || height == 0 {
                return;
            }

            let mut hasher = Sha256::new();
            hasher.update(rgba_bytes);
            hasher.update(&width.to_le_bytes());
            hasher.update(&height.to_le_bytes());
            let hash = format!("{:x}", hasher.finalize());

            if state.last_image_hash.as_deref() != Some(&hash) {
                log::info!("检测到新图片 ({}x{})", width, height);
                state.last_image_hash = Some(hash.clone());
                state.last_text = None;

                match images::save_image(&db.data_dir, rgba_bytes, width, height) {
                    Ok(filename) => {
                        let item = ClipItem::new_image(0, filename, hash, now);
                        match db.insert_clip(&item) {
                            Ok(new_id) => {
                                let mut item_with_id = item.clone();
                                item_with_id.id = new_id;
                                log::info!("图片已存储，id={}", new_id);
                                let _ = app_handle.emit("clipboard-changed", &item_with_id);
                            }
                            Err(e) => log::error!("存储图片条目失败: {}", e),
                        }
                    }
                    Err(e) => log::error!("保存图片文件失败: {}", e),
                }
            }
        }
        Err(ref e) => {
            log::debug!("read_image 无图片: {:?}", e);
        }
    }
}
