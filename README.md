<div align="center">

# 🦊 Fox AI Studio

**An Open-Source AI Agent Desktop with Vision-Based Computer Control**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://v2.tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3.5-42b883.svg)](https://vuejs.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)

[English](#features) · [中文](#功能特性) · [Getting Started](#getting-started) · [Architecture](#architecture) · [Contributing](#contributing)

</div>

---

Fox AI Studio is a cross-platform desktop AI agent that can **see your screen, click, type, and complete complex multi-step tasks autonomously**. Built with Tauri 2 + Vue 3 + Rust, it combines a vision-based agent loop with precise desktop control — enabling AI to operate your computer like a human.

> 🎯 Click desktop icons, drag chess pieces, fill out forms, navigate applications — Fox AI does it all through screenshot analysis and pixel-perfect mouse/keyboard control.

## ✨ Features

- **🖥️ Computer Use Agent** — Autonomous vision-action loop: screenshot → AI analysis → mouse/keyboard operation → verify, repeating until the task is done
- **🎯 Pixel-Perfect Positioning** — Coordinate ruler overlays, desktop icon enumeration via Win32 Shell API, and DPI-aware coordinate mapping for precise clicking
- **🔄 Persistent Task Execution** — Up to 200-step autonomous loops with state tracking, enabling complex workflows like "play this chess game for me"
- **🛠️ 27+ Built-in Tools** — Screenshot, mouse click/drag/scroll, keyboard input, terminal, file operations, web fetching, code execution, and more
- **🤖 Multi-Provider Support** — Works with OpenAI-compatible APIs, Anthropic, and any provider supporting function calling / tool use
- **🔄 API Format Conversion** — Built-in Axum proxy for bidirectional conversion between OpenAI ↔ Anthropic API formats
- **📝 Rich Chat Interface** — Markdown rendering, code highlighting, LaTeX math, Mermaid diagrams, and tool-call visualization
- **🧠 Knowledge & Memory** — Persistent knowledge base, user memory, skill management, and session search
- **🌍 i18n Ready** — English and Chinese out of the box
- **⚡ Blazing Fast** — Rust backend for screen capture (xcap), input simulation (enigo), and clipboard — no Electron overhead

## 📸 Screenshots

> *Coming soon — contributions welcome!*

## 🚀 Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) ≥ 18
- [pnpm](https://pnpm.io/) ≥ 8
- [Rust](https://www.rust-lang.org/tools/install) ≥ 1.77.2
- [Tauri 2 CLI](https://v2.tauri.app/start/prerequisites/) — platform-specific dependencies

### Install & Run

```bash
# Clone the repository
git clone https://github.com/nifeng6/fox-ai-studio.git
cd fox-ai-studio

# Install frontend dependencies
pnpm install

# Start in development mode
pnpm tauri:dev
```

### Build for Production

```bash
pnpm tauri:build
```

The installer will be generated in `src-tauri/target/release/bundle/`.

### Configure AI Provider

1. Launch Fox AI Studio
2. Go to **Settings → Provider**
3. Enter your API endpoint and key (OpenAI-compatible format supported)
4. Select a model with vision + function calling capabilities
5. Start chatting with Computer Use mode enabled

## 🏗️ Architecture

```
fox-ai-studio/
├── src/                          # Frontend (Vue 3 + TypeScript)
│   ├── pages/                    # 12 page components
│   │   ├── ChatPage.vue          # Main chat interface
│   │   ├── ComputerUsePage.vue   # Computer Use agent panel
│   │   ├── AgentPage.vue         # Agent configuration
│   │   ├── KnowledgePage.vue     # Knowledge base
│   │   ├── SettingsPage.vue      # App settings & provider config
│   │   └── ...
│   ├── components/
│   │   ├── chat/                 # Chat UI, message list, tool call cards
│   │   ├── common/               # Layout, sidebar, titlebar
│   │   └── markdown/             # Rich markdown renderer
│   ├── stores/                   # 15 Pinia stores (state management)
│   ├── utils/
│   │   ├── tool-executor.ts      # Frontend tool execution pipeline
│   │   ├── harness.ts            # Agent test harness
│   │   └── tauri-api.ts          # Tauri IPC bindings
│   └── i18n/                     # Internationalization
│
└── src-tauri/                    # Backend (Rust + Tauri 2)
    ├── src/
    │   ├── commands/
    │   │   ├── agent_loop.rs      # 🧠 Vision-action agent loop
    │   │   ├── desktop.rs         # 📸 Screenshot + coordinate ruler + icon enumeration
    │   │   ├── input.rs           # 🖱️ Mouse/keyboard control with DPI mapping
    │   │   ├── tools.rs           # 🔧 27+ tool registry & execution
    │   │   ├── chat.rs            # Chat message handling
    │   │   ├── proxy.rs           # API proxy server
    │   │   ├── provider.rs        # LLM provider management
    │   │   └── ...
    │   └── proxy/                 # OpenAI ↔ Anthropic format converter
    └── Cargo.toml
```

### How the Agent Loop Works

```
┌─────────────────────────────────────────────────┐
│                  User Request                    │
│            "Help me win this chess game"          │
└──────────────────────┬──────────────────────────┘
                       ▼
              ┌─────────────────┐
              │   Screenshot     │◄──────────────────┐
              │  (xcap capture)  │                    │
              └────────┬────────┘                    │
                       ▼                             │
              ┌─────────────────┐                    │
              │  Send to LLM     │                    │
              │  (vision + tools)│                    │
              └────────┬────────┘                    │
                       ▼                             │
              ┌─────────────────┐                    │
              │  Parse Action    │                    │
              │  (tool calls)    │                    │
              └────────┬────────┘                    │
                       ▼                             │
              ┌─────────────────┐    Task not done   │
              │  Execute Action  │────────────────────┘
              │  (click/type/drag)│
              └────────┬────────┘
                       │ Task complete
                       ▼
              ┌─────────────────┐
              │   Notify User    │
              └─────────────────┘
```

## 🛠️ Tool Catalog

| Category | Tools | Description |
|----------|-------|-------------|
| **Computer Use** | `screenshot`, `mouse_click`, `mouse_double_click`, `mouse_move`, `mouse_drag`, `mouse_scroll`, `keyboard_type`, `keyboard_key`, `keyboard_hotkey`, `open_application`, `action_sequence`, `wait` | Full screen control with DPI-aware coordinate mapping |
| **Terminal** | `terminal`, `process` | Persistent shell sessions with background execution |
| **File** | `read_file`, `write_file`, `patch`, `search_files` | Read, write, patch, and search files |
| **Web** | `web_search`, `web_extract`, `fetch_url` | Search the web and extract content |
| **Vision** | `vision_analyze` | Dedicated vision AI for image analysis |
| **Code** | `execute_code` | Run Python, JavaScript, Shell snippets |
| **Planning** | `task_complete`, `todo`, `session_search`, `memory` | Task management and persistent memory |
| **Skills** | `skills_list`, `skill_view`, `skill_manage` | Create and reuse skill templates |

## 🔧 Tech Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Frontend** | Vue 3 + TypeScript + Vite | Reactive UI with Composition API |
| **UI Framework** | Element Plus + TipTap | Rich components and text editing |
| **State** | Pinia + Persisted State | 15 stores with localStorage persistence |
| **Desktop** | Tauri 2 | Lightweight native shell |
| **Backend** | Rust | Screen capture, input, API proxy |
| **Screen Capture** | xcap | Cross-platform screenshot |
| **Input Simulation** | enigo | Mouse & keyboard control |
| **Clipboard** | arboard | Clipboard read/write |
| **API Proxy** | Axum | OpenAI ↔ Anthropic conversion |
| **i18n** | vue-i18n | Multi-language support |

## 🤝 Contributing

Contributions are welcome! Whether it's a bug fix, feature request, or documentation improvement — every PR matters.

1. **Fork** the repository
2. Create a **feature branch**: `git checkout -b feat/my-feature`
3. **Commit** your changes: `git commit -m 'feat: add my feature'`
4. **Push** to the branch: `git push origin feat/my-feature`
5. Open a **Pull Request**

Please make sure to:
- Follow the existing code style
- Test your changes before submitting
- Update documentation if needed

### Development Setup

```bash
# Install dependencies
pnpm install

# Run in dev mode with hot reload
pnpm tauri:dev

# Type check frontend
pnpm vue-tsc --noEmit

# Build for production
pnpm tauri:build
```

## 📄 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) — for the incredible Rust-based desktop framework
- [xcap](https://github.com/nicehash/xcap) — cross-platform screen capture in Rust
- [enigo](https://github.com/enigo-rs/enigo) — input simulation library
- [Vue.js](https://vuejs.org/) — the progressive JavaScript framework
- [Element Plus](https://element-plus.org/) — Vue 3 component library

---

<div align="center">

**[⬆ Back to Top](#-fox-ai-studio)**

Made with ❤️ by the Fox AI community

</div>

---

<details>
<summary>📖 中文文档</summary>

## 🦊 Fox AI Studio

**开源 AI 智能体桌面应用 — 基于视觉的计算机控制**

Fox AI Studio 是一个跨平台桌面 AI 智能体，能够**看见你的屏幕、点击、打字、并自主完成复杂的多步骤任务**。基于 Tauri 2 + Vue 3 + Rust 构建，结合视觉驱动的智能体循环与精确的桌面控制——让 AI 像人一样操作你的电脑。

> 🎯 点击桌面图标、拖动棋子、填写表单、操作应用程序——Fox AI 通过截图分析和像素级键鼠控制完成一切。

### ✨ 功能特性

- **🖥️ Computer Use 智能体** — 自主视觉-动作循环：截图 → AI 分析 → 键鼠操作 → 验证，循环直至任务完成
- **🎯 像素级精准定位** — 坐标刻度尺叠加、Win32 Shell API 桌面图标枚举、DPI 感知坐标映射
- **🔄 持续任务执行** — 最多 200 步自主循环，支持"帮我下完这盘棋"等复杂任务
- **🛠️ 27+ 内置工具** — 截图、鼠标点击/拖拽/滚动、键盘输入、终端、文件操作、网页抓取、代码执行等
- **🤖 多模型支持** — 兼容 OpenAI 格式 API、Anthropic 及任何支持函数调用的模型
- **🔄 API 格式转换** — 内置 Axum 代理，支持 OpenAI ↔ Anthropic 格式双向转换
- **📝 富文本聊天** — Markdown 渲染、代码高亮、LaTeX 数学公式、Mermaid 图表、工具调用可视化
- **🧠 知识与记忆** — 持久化知识库、用户记忆、技能管理、会话搜索
- **🌍 国际化** — 开箱即用的中英文支持
- **⚡ 极致性能** — Rust 后端处理截图（xcap）、输入模拟（enigo）、剪贴板——无 Electron 开销

### 🚀 快速开始

#### 环境要求

- [Node.js](https://nodejs.org/) ≥ 18
- [pnpm](https://pnpm.io/) ≥ 8
- [Rust](https://www.rust-lang.org/tools/install) ≥ 1.77.2
- [Tauri 2 依赖](https://v2.tauri.app/start/prerequisites/)

#### 安装与运行

```bash
# 克隆仓库
git clone https://github.com/nifeng6/fox-ai-studio.git
cd fox-ai-studio

# 安装前端依赖
pnpm install

# 开发模式启动
pnpm tauri:dev
```

#### 生产构建

```bash
pnpm tauri:build
```

安装包将生成在 `src-tauri/target/release/bundle/` 目录下。

#### 配置 AI 提供商

1. 启动 Fox AI Studio
2. 进入 **设置 → 提供商**
3. 输入 API 地址和密钥（支持 OpenAI 兼容格式）
4. 选择支持视觉 + 函数调用的模型
5. 开启 Computer Use 模式开始使用

### 🤝 参与贡献

欢迎各种形式的贡献！无论是 Bug 修复、新功能还是文档改进——每一个 PR 都很重要。

1. **Fork** 本仓库
2. 创建 **功能分支**：`git checkout -b feat/my-feature`
3. **提交** 更改：`git commit -m 'feat: add my feature'`
4. **推送** 分支：`git push origin feat/my-feature`
5. 提交 **Pull Request**

### 📄 许可证

本项目基于 MIT 许可证开源 — 详见 [LICENSE](LICENSE) 文件。

</details>
