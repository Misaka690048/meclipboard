use std::sync::Arc;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_autostart::ManagerExt;

use crate::clipboard::models::{ClipItem, ContentType, Settings, StorageStats};
use crate::storage::database::Database;
use crate::storage::images;

/// 获取历史记录列表
#[tauri::command]
pub fn get_history(db: State<'_, Arc<Database>>, limit: Option<i64>) -> Result<Vec<ClipItem>, String> {
    db.get_history(limit).map_err(|e| e.to_string())
}

/// 搜索文字内容
#[tauri::command]
pub fn search_clips(db: State<'_, Arc<Database>>, query: String) -> Result<Vec<ClipItem>, String> {
    if query.trim().is_empty() {
        return db.get_history(Some(200)).map_err(|e| e.to_string());
    }
    db.search_clips(&query).map_err(|e| e.to_string())
}

/// 切换置顶状态，返回新状态
#[tauri::command]
pub fn toggle_pin(db: State<'_, Arc<Database>>, id: i64) -> Result<bool, String> {
    db.toggle_pin(id).map_err(|e| e.to_string())
}

/// 删除条目（图片文件保留，由清理任务统一处理）
#[tauri::command]
pub fn delete_item(db: State<'_, Arc<Database>>, id: i64) -> Result<(), String> {
    db.delete_clip(id).map_err(|e| e.to_string())?;
    Ok(())
}

/// 将条目内容恢复到系统剪贴板
#[tauri::command]
pub fn restore_to_clipboard(
    app_handle: AppHandle,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    let item = db
        .get_clip_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("条目不存在")?;

    match item.content_type {
        ContentType::Text => {
            let text = item.text_content.unwrap_or_default();
            app_handle
                .clipboard()
                .write_text(&text)
                .map_err(|e| format!("写入文字失败: {}", e))?;
        }
        ContentType::Image => {
            let image_filename = item.image_path.unwrap_or_default();
            if !image_filename.is_empty() {
                let (rgba, width, height) =
                    images::load_image(&db.data_dir, &image_filename)
                        .map_err(|e| format!("加载图片失败: {}", e))?;

                let tauri_image = tauri::image::Image::new(&rgba, width, height);
                app_handle
                    .clipboard()
                    .write_image(&tauri_image)
                    .map_err(|e| format!("写入图片失败: {}", e))?;
            }
        }
    }

    Ok(())
}

/// 获取所有设置
#[tauri::command]
pub fn get_settings(db: State<'_, Arc<Database>>) -> Result<Settings, String> {
    db.get_all_settings().map_err(|e| e.to_string())
}

/// 更新设置
#[tauri::command]
pub fn update_settings(
    app_handle: AppHandle,
    db: State<'_, Arc<Database>>,
    settings: Settings,
) -> Result<(), String> {
    db.set_setting("retention_days", &settings.retention_days.to_string())
        .map_err(|e| e.to_string())?;
    db.set_setting("storage_cap_mb", &settings.storage_cap_mb.to_string())
        .map_err(|e| e.to_string())?;
    db.set_setting("autostart", &settings.autostart.to_string())
        .map_err(|e| e.to_string())?;

    // 开机自启（由 Rust 端直接操作 autolaunch）
    let am = app_handle.autolaunch();
    if settings.autostart {
        am.enable().map_err(|e| format!("启用开机自启失败: {}", e))?;
    } else {
        let _ = am.disable();
    }

    Ok(())
}

/// 获取图片的 base64 data URL（用于前端显示）
#[tauri::command]
pub fn get_image_data_url(db: State<'_, Arc<Database>>, id: i64) -> Result<String, String> {
    let item = db
        .get_clip_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("条目不存在")?;

    let filename = item.image_path.unwrap_or_default();
    if filename.is_empty() {
        return Err("非图片条目".into());
    }

    let filepath = db.data_dir.join("images").join(&filename);
    let bytes = std::fs::read(&filepath)
        .map_err(|e| format!("读取图片文件失败: {}", e))?;
    let encoded = BASE64.encode(&bytes);
    Ok(format!("data:image/png;base64,{}", encoded))
}

/// 获取存储统计
#[tauri::command]
pub fn get_storage_stats(db: State<'_, Arc<Database>>) -> Result<StorageStats, String> {
    let (item_count, text_bytes) = db.get_storage_stats().map_err(|e| e.to_string())?;
    let image_size = images::images_dir_size(&db.data_dir);

    Ok(StorageStats {
        total_bytes: text_bytes + image_size,
        item_count,
        image_count: 0,
    })
}

/// 撤销删除：重新插入已删除的条目
#[tauri::command]
pub fn restore_deleted_item(
    db: State<'_, Arc<Database>>,
    item: ClipItem,
) -> Result<i64, String> {
    let new_item = ClipItem {
        id: 0,
        content_type: item.content_type,
        text_content: item.text_content,
        image_path: item.image_path,
        image_hash: item.image_hash,
        created_at: item.created_at,
        is_pinned: item.is_pinned,
    };
    db.insert_clip(&new_item).map_err(|e| e.to_string())
}
