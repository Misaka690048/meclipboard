use std::path::PathBuf;
use sha2::{Sha256, Digest};

/// 将图片的 RGBA 原始字节保存为 PNG 文件
/// 返回：相对于 data_dir 的文件路径
pub fn save_image(data_dir: &PathBuf, rgba_bytes: &[u8], width: u32, height: u32) -> Result<String, String> {
    let images_dir = data_dir.join("images");
    std::fs::create_dir_all(&images_dir).map_err(|e| format!("创建图片目录失败: {}", e))?;

    // 用 SHA-256 哈希命名，自动去重
    let mut hasher = Sha256::new();
    hasher.update(rgba_bytes);
    hasher.update(&width.to_le_bytes());
    hasher.update(&height.to_le_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let filename = format!("{}.png", hash);
    let filepath = images_dir.join(&filename);

    // 如果文件已存在，跳过写入
    if !filepath.exists() {
        // RGBA → RgbaImage → PNG
        let img = image::RgbaImage::from_raw(width, height, rgba_bytes.to_vec())
            .ok_or("无法从原始字节构建图片")?;
        img.save(&filepath).map_err(|e| format!("保存图片失败: {}", e))?;
    }

    Ok(filename)
}

/// 从磁盘加载图片为 RGBA 字节
pub fn load_image(data_dir: &PathBuf, filename: &str) -> Result<(Vec<u8>, u32, u32), String> {
    let filepath = data_dir.join("images").join(filename);
    let img = image::open(&filepath).map_err(|e| format!("打开图片失败: {}", e))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((rgba.into_raw(), w, h))
}

/// 删除图片文件
pub fn delete_image_file(data_dir: &PathBuf, filename: &str) {
    let filepath = data_dir.join("images").join(filename);
    if let Err(e) = std::fs::remove_file(&filepath) {
        log::warn!("删除图片文件失败 {}: {}", filepath.display(), e);
    }
}

/// 计算图片目录总大小（字节）
pub fn images_dir_size(data_dir: &PathBuf) -> u64 {
    let images_dir = data_dir.join("images");
    if !images_dir.exists() {
        return 0;
    }
    let mut total: u64 = 0;
    if let Ok(entries) = std::fs::read_dir(&images_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}
