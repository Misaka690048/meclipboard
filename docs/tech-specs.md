# 技术规格说明书

## 1. 技术栈

| 层级 | 技术 |
|------|------|
| **桌面框架** | Tauri v2（Rust 后端 + WebView2 前端） |
| **前端** | 原生 HTML/CSS/JavaScript（无框架） |
| **后端** | Rust（Tokio 异步运行时） |
| **数据库** | SQLite（via rusqlite crate, bundled 模式） |
| **图片处理** | Rust `image` crate |
| **哈希去重** | Rust `sha2` crate（SHA-256） |

---

## 2. 关键依赖及版本

### 2.1 Rust Cargo 依赖（src-tauri/Cargo.toml）

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-clipboard-manager = "2"
tauri-plugin-global-shortcut = "2"
tauri-plugin-autostart = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
tokio = { version = "1", features = ["full"] }
image = "0.25"
sha2 = "0.10"
chrono = "0.4"
```

### 2.2 npm 依赖（package.json）

```json
{
  "devDependencies": {
    "@tauri-apps/cli": "^2"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-clipboard-manager": "^2",
    "@tauri-apps/plugin-global-shortcut": "^2",
    "@tauri-apps/plugin-autostart": "^2"
  }
}
```

---

## 3. 数据模型

### 3.1 clips 表

```sql
CREATE TABLE IF NOT EXISTS clips (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    content_type TEXT    NOT NULL,   -- 'text' 或 'image'
    text_content TEXT,               -- 文字内容（content_type='text' 时有值）
    image_path   TEXT,               -- 图片相对路径（content_type='image' 时有值）
    image_hash   TEXT,               -- 图片 SHA-256 哈希（16进制，64字符）
    created_at   INTEGER NOT NULL,   -- Unix 时间戳（秒）
    is_pinned    INTEGER DEFAULT 0   -- 0=未置顶, 1=置顶
);

CREATE INDEX IF NOT EXISTS idx_clips_created_at ON clips(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_clips_is_pinned ON clips(is_pinned);
```

### 3.2 settings 表

```sql
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- 默认值
INSERT OR IGNORE INTO settings (key, value) VALUES ('retention_days', '30');
INSERT OR IGNORE INTO settings (key, value) VALUES ('storage_cap_mb', '500');
INSERT OR IGNORE INTO settings (key, value) VALUES ('autostart', 'false');
```

### 3.3 Rust 数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: i64,
    pub content_type: String,    // "text" | "image"
    pub text_content: Option<String>,
    pub image_path: Option<String>,
    pub created_at: i64,         // Unix timestamp (seconds)
    pub is_pinned: bool,
}
```

---

## 4. IPC 通信设计

### 4.1 Tauri Commands（前端 invoke 调用后端）

| 命令 | 方向 | 参数 | 返回值 |
|------|------|------|--------|
| `get_history` | JS→Rust | `limit: Option<i64>` | `Vec<ClipItem>` |
| `search_clips` | JS→Rust | `query: String` | `Vec<ClipItem>` |
| `pin_item` | JS→Rust | `id: i64` | `()` |
| `unpin_item` | JS→Rust | `id: i64` | `()` |
| `delete_item` | JS→Rust | `id: i64` | `()` |
| `restore_to_clipboard` | JS→Rust | `id: i64` | `()` |
| `get_settings` | JS→Rust | — | `Settings` |
| `update_settings` | JS→Rust | `Settings` | `()` |
| `get_storage_stats` | JS→Rust | — | `StorageStats` |

### 4.2 Tauri Events（后端推送前端）

| 事件名 | 方向 | 载荷 |
|------|------|------|
| `clipboard-changed` | Rust→JS | `ClipItem`（新记录的条目） |

---

## 5. 剪贴板监听架构

```
┌──────────────────────────────────────────────┐
│  tokio::spawn(async move {                    │
│    loop {                                     │
│      tokio::time::sleep(500ms);               │
│      text = app.clipboard().read_text();      │
│      image = app.clipboard().read_image();    │
│      if changed(text, image, &last_state) {   │
│        item = store_to_db(...);               │
│        emit("clipboard-changed", item);        │
│      }                                        │
│    }                                          │
│  })                                           │
└──────────────────────────────────────────────┘
```

---

## 6. 存储路径

```
{app_local_data_dir}/
├── clipboard.db
└── images/
    └── {sha256_hex}.png
```

- Windows 实际路径：`C:\Users\{user}\AppData\Local\com.clipboard-app.app\`
- 使用 Tauri 的 `app.path().app_local_data_dir()` 获取

---

## 7. 开发命令

```bash
npm run tauri dev      # 开发模式（热重载）
npm run tauri build    # 生产构建
npm run tauri build -- --debug  # 调试构建
```

在 `src-tauri/` 目录下：

```bash
cargo check            # 快速检查 Rust 编译（不生成二进制）
cargo build            # 编译 Rust 部分
```
