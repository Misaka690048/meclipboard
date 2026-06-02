<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="MeClipboard 图标" width="128" height="128">
</p>

<h1 align="center">MeClipboard</h1>

<p align="center">
  <strong>轻量、美观、纯本地的 Windows 剪贴板历史管理工具</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/平台-Windows_10/11-blue?logo=windows" alt="平台">
  <img src="https://img.shields.io/badge/框架-Tauri_v2-FFC131?logo=tauri" alt="Tauri">
  <img src="https://img.shields.io/badge/语言-Rust_+_Vanilla_JS-DEA584?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/许可-MIT-green" alt="许可">
</p>

---

## 简介

**MeClipboard** 是一款 Windows 剪贴板历史管理工具，后台静默记录你复制的每一段文字和每一张截图。需要时，按下 `Alt+V` 呼出窗口，搜索、置顶、回贴，一切都在指尖。

- **纯本地** — 数据全部存储在本机 SQLite + 文件中，不上传、不联网、无遥测
- **轻量高效** — 基于 Tauri v2，Rust 后端 + 原生 JS 前端，无臃肿框架
- **米白色主题** — 简洁卡片式 UI，视觉舒适

---

## 功能

| 功能 | 说明 |
|------|------|
| 自动记录 | 复制文字或截图后自动存入历史，全程后台静默 |
| 卡片界面 | 米白色主题，置顶卡片带金色标记常驻顶部 |
| 搜索过滤 | 输入关键词实时筛选文字记录 |
| 置顶与删除 | 重要内容一键置顶不被清理；删除支持 3 秒撤销恢复 |
| 回贴 | 点击卡片恢复内容到系统剪贴板，Ctrl+V 直接粘贴 |
| 可配置策略 | 自定义保留天数和存储上限，置顶项永不自动清理 |
| 系统托盘 | 关闭窗口缩至托盘，后台持续记录 |
| 全局快捷键 | `Alt+V` 随时随地呼出窗口 |
| 开机自启 | 可选开机启动，启动后静默在托盘，不弹出窗口 |

---

## 技术栈

| 层 | 技术 |
|------|------|
| 桌面框架 | [Tauri v2](https://v2.tauri.app/) (Rust + WebView2) |
| 后端 | Rust |
| 前端 | 原生 HTML / CSS / JavaScript |
| 数据库 | SQLite（`rusqlite`，bundled 模式） |
| 剪贴板 | `tauri-plugin-clipboard-manager` |
| 图片处理 | `image` crate（PNG 编解码，SHA-256 哈希去重） |
| 打包 | NSIS（`.exe` 安装程序） |

---

## 安装

1. 从 [Releases](../../releases) 下载 `meclipboard_*_x64-setup.exe`
2. 双击运行，按向导完成安装
3. 复制一段文字或截图试试——卡片自动出现了

> 系统要求：Windows 10/11 (x64)，WebView2 运行时（Win10+ 已预装）

---

## 开发指南

### 环境准备

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/) 1.77+
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)（Windows 必需）

### 克隆项目

```bash
git clone https://github.com/Misaka690048/meclipboard.git
cd meclipboard
npm install
```

### 开发模式

```bash
npm run tauri dev
```

窗口打开后，复制文字或截图——卡片自动出现在列表中。

### 生产构建

```bash
npm run tauri build
```

安装包生成在 `src-tauri/target/release/bundle/nsis/meclipboard_*_x64-setup.exe`。

---

## 架构

```
+------------------------------------------+
|               前端 (WebView2)              |
|  index.html  main.js  history.js          |
|  settings.js  CSS (BEM 命名)              |
|                                            |
|  - 每 1 秒轮询 get_history 获取新记录       |
|  - 事件委托处理卡片点击（置顶/删除/回贴）     |
|  - 250ms 防抖搜索                          |
+------------------+-----------------------+
                   |  invoke() IPC
+------------------v-----------------------+
|              Rust 后端 (Tauri)             |
|                                            |
|  lib.rs         应用启动、托盘、快捷键        |
|  commands.rs    10 个 Tauri 命令            |
|  monitor.rs     剪贴板轮询 (std::thread)    |
|  database.rs    SQLite CRUD                |
|  images.rs      PNG 图片存取               |
+------------------+-----------------------+
                   |
+------------------v-----------------------+
|              本地存储                       |
|                                            |
|  %LOCALAPPDATA%/meclipboard/               |
|  +-- clipboard.db   SQLite 数据库           |
|  +-- images/        PNG 图片文件            |
+------------------------------------------+
```

---

## 项目结构

```
meclipboard/
├── index.html                # 应用入口页面
├── package.json              # npm 配置
├── README.md                 # 项目文档
├── src/
│   ├── css/
│   │   ├── main.css          # 全局样式 + 米白色主题
│   │   ├── history.css       # 卡片列表布局
│   │   └── settings.css      # 设置面板样式
│   └── js/
│       ├── main.js           # 初始化、轮询、IPC 封装
│       ├── history.js        # 卡片渲染、置顶/删除/回贴
│       └── settings.js       # 设置面板逻辑
├── src-tauri/
│   ├── Cargo.toml            # Rust 依赖
│   ├── tauri.conf.json       # Tauri 窗口/打包/安全配置
│   ├── capabilities/
│   │   └── default.json      # 权限声明
│   ├── icons/                # 应用图标
│   └── src/
│       ├── main.rs           # Rust 入口
│       ├── lib.rs            # 应用启动、插件、托盘、快捷键
│       ├── commands.rs       # 全部 IPC 命令
│       ├── clipboard/
│       │   ├── monitor.rs    # 剪贴板轮询 (500ms)
│       │   └── models.rs     # ClipItem / Settings / ContentType
│       └── storage/
│           ├── database.rs   # SQLite 建表与 CRUD
│           └── images.rs     # 图片文件 I/O
└── docs/
    ├── requirements.md       # 功能需求文档
    ├── tech-specs.md         # 技术规格文档
    └── design-specs.md       # UI 设计规范
```

---

## 设计规范

| 元素 | 色值 | 说明 |
|------|------|------|
| 背景 | `#FFF8F0` | 米白色主背景 |
| 卡片 | `#FAF0E6` | 卡片底色 |
| 边框 | `#DEB887` | 卡片描边 |
| 强调 | `#CD853F` | 秘鲁棕（按钮、置顶标记） |
| 文字 | `#3E2723` | 深棕色正文 |
| 窗口 | 420×640 | 默认尺寸，可调大小 |

详见 [docs/design-specs.md](docs/design-specs.md)

---

## 许可

MIT (c) 2026 Misaka690048
