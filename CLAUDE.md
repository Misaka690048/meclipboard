# CLAUDE.md — ClipHistory 剪贴板历史应用

## 项目信息

- **类型**：Tauri v2 Windows 桌面应用
- **前端**：原生 HTML/CSS/JavaScript（`src/`）
- **后端**：Rust（`src-tauri/`）
- **存储**：SQLite + 本地文件系统（纯本地，无网络）

---

## 开发命令

```bash
npm run tauri dev          # 开发模式（热重载窗口）
npm run tauri build        # 生产构建 .msi
cargo check                # 快速检查 Rust 编译（在 src-tauri/ 下运行）
```

---

## 项目标准文件

所有规范文档位于 `docs/`，开发时务必遵循：

| 文件 | 内容 | 何时查阅 |
|------|------|----------|
| [`docs/requirements.md`](docs/requirements.md) | 功能需求 + 验收标准 | 不确定功能边界时 |
| [`docs/tech-specs.md`](docs/tech-specs.md) | 技术栈、数据模型、IPC 设计 | 写后端代码时 |
| [`docs/design-specs.md`](docs/design-specs.md) | UI 配色、布局、字体、交互 | 写前端样式时 |
| [`docs/execution-steps.md`](docs/execution-steps.md) | 5 阶段分步执行清单 | 规划当次会话任务时 |

---

## 开发日志

每次开发会话结束后，更新 `dev-log/YYYY-MM-DD.md`，格式：

```markdown
# Dev Log -- YYYY-MM-DD

## 完成事项
- [x] 具体完成的任务

## 下次待办
- [ ] 下次要做的任务

## 备注
其他需要记录的信息
```

---

## 架构要点

```
剪贴板轮询 (Rust tokio 500ms)
    ↓ 检测到变化
SQLite 存储
    ↓ 发送事件
前端监听 "clipboard-changed" → 更新卡片列表
```

- 剪贴板轮询在 `src-tauri/src/clipboard/monitor.rs`，启动时 spawn 为 tokio 后台任务
- 前端通过 Tauri events 接收更新，通过 Tauri commands（`invoke`）发起操作
- 系统托盘、全局快捷键在 `src-tauri/src/lib.rs` 中配置
- 设置项存储在 SQLite `settings` 表（key-value）
- 图片文件存储在 `{app_local_data_dir}/images/{hash}.png`

---

## 不允许修改的文件

- `src-tauri/src/main.rs` — Tauri 入口，保持最小化
- `src-tauri/build.rs` — Tauri 标准构建脚本

---

## 编码约定

### Rust
- 所有 `#[tauri::command]` 统一放在 `commands.rs`
- 模块按功能划分：`clipboard/`（监听）、`storage/`（数据库+文件）
- 使用 `anyhow::Result` 风格的错误处理

### JavaScript
- 原生 JS，不使用任何框架
- 每个文件负责一项功能：`history.js`、`search.js`、`operations.js`、`settings.js`
- `main.js` 负责初始化 + DOMContentLoaded + 事件监听注册

### CSS
- BEM 风格命名：`.card`、`.card__pin-btn`、`.card__content`
- 颜色值使用设计规范中已定义的色值（见 `docs/design-specs.md`）
