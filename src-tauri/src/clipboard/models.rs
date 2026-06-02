use serde::{Deserialize, Serialize};

/// 内容类型枚举（类型安全，替代魔术字符串）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Text,
    Image,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Text => "text",
            ContentType::Image => "image",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(ContentType::Text),
            "image" => Some(ContentType::Image),
            _ => None,
        }
    }
}

/// 剪贴板条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: i64,
    pub content_type: ContentType,
    pub text_content: Option<String>,
    pub image_path: Option<String>,
    #[serde(default)]
    pub image_hash: Option<String>,
    pub created_at: i64,            // Unix 时间戳（秒）
    #[serde(default)]
    pub is_pinned: bool,
}

impl ClipItem {
    /// 创建新的文字条目
    pub fn new_text(id: i64, text: String, timestamp: i64) -> Self {
        Self {
            id,
            content_type: ContentType::Text,
            text_content: Some(text),
            image_path: None,
            image_hash: None,
            created_at: timestamp,
            is_pinned: false,
        }
    }

    /// 创建新的图片条目
    pub fn new_image(id: i64, image_path: String, image_hash: String, timestamp: i64) -> Self {
        Self {
            id,
            content_type: ContentType::Image,
            text_content: None,
            image_path: Some(image_path),
            image_hash: Some(image_hash),
            created_at: timestamp,
            is_pinned: false,
        }
    }
}

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub retention_days: u32,
    pub storage_cap_mb: u32,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            retention_days: 30,
            storage_cap_mb: 500,
            autostart: false,
        }
    }
}

/// 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_bytes: u64,
    pub item_count: u64,
    pub image_count: u64,
}
