# 分步执行计划

> 每个阶段完成后可独立运行验证，确认无误后再进入下一阶段。

---

## 阶段 1：项目脚手架 + Hello World

- [x] 创建项目目录结构
- [x] 编写 docs/ 标准文件
- [x] 编写 CLAUDE.md
- [x] `npm init` + 安装 Tauri CLI + `npx tauri init`
- [x] 创建 `index.html` + `src/css/main.css`（米白色背景）
- [x] 创建 `src/js/main.js`（基础初始化）
- [x] 验证：`npm run tauri dev` 成功启动，窗口显示米白色背景

---

## 阶段 2：剪贴板监听 + 存储 + 卡片展示

- [x] Rust 依赖添加到 `Cargo.toml`
- [x] 实现 `clipboard/models.rs`（ClipItem 数据结构）
- [x] 实现 `storage/database.rs`（SQLite 建表 + CRUD）
- [x] 实现 `storage/images.rs`（图片保存/加载）
- [x] 实现 `clipboard/monitor.rs`（轮询循环 500ms）
- [x] 实现 `commands.rs`（get_history, get_clip_count）
- [x] 实现 `lib.rs`（插件注册 + 后台任务启动）
- [x] 配置 `tauri.conf.json` + `capabilities/default.json`
- [x] 前端 `history.js`：渲染卡片列表 + 轮询检测新条目
- [x] 前端 `history.css`：卡片样式
- [x] 验证：复制文字和图片，窗口中自动出现卡片

---

## 阶段 3：搜索、置顶、删除、回贴

- [x] 实现 `search_clips` 命令（后端 SQL 模糊搜索）
- [x] 实现 `toggle_pin` 命令
- [x] 实现 `delete_item` 命令（含图片文件清理）
- [x] 实现 `restore_to_clipboard` 命令（文字 + 图片回贴）
- [x] 前端搜索栏 + 防抖过滤（250ms）
- [x] 前端置顶、删除+撤销、回贴操作 + 视觉反馈
- [x] 验证：所有卡片操作正常工作 + 持久化

---

## 阶段 4：设置面板 + 自动清理

- [x] 实现保留天数清理 + 容量上限清理
- [x] 实现 `get_settings` / `update_settings` / `get_storage_stats` 命令
- [x] 前端 `settings.js`：设置面板 UI（右侧抽屉）
- [x] 前端 `settings.css`：设置面板样式
- [x] 启动时 + 每小时执行清理任务
- [x] 验证：修改保留天数后清理生效；超容量后自动删除旧项

---

## 阶段 5：系统托盘 + Alt+V 快捷键 + 开机自启

- [x] 系统托盘图标 + 右键菜单（"显示" / "退出"）+ 左键点击显示
- [x] 关闭窗口 → 隐藏到托盘（prevent_close + hide）
- [x] 注册 Alt+V 全局快捷键 → unminimize + show + focus
- [x] 开机自启功能（Rust 端 autolaunch enable/disable）
- [x] 设置面板中开机自启开关
- [x] 验证：托盘菜单、Alt+V、开机自启均正常工作

---

## 阶段 6：打磨 + 打包

- [x] 生成专属应用图标（32x32 / 128x128 / 256x256 PNG + .ico）
- [x] 修改 `tauri.conf.json` targets 为 `["nsis"]`（.exe 安装包）
- [x] `npm run tauri build` 生产构建成功
- [x] 安装包：`cliphistory_0.1.0_x64-setup.exe`
- [x] 可执行文件：`cliphistory.exe`

---

## 构建产物

| 文件 | 路径 |
|------|------|
| .exe 安装包 | `src-tauri/target/release/bundle/nsis/cliphistory_0.1.0_x64-setup.exe` |
| 可执行文件 | `src-tauri/target/release/cliphistory.exe` |

---

**项目开发完成** ✅
