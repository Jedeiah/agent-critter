# Agent Critter 🐱

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Petdex](https://img.shields.io/badge/pets-2700%2B-ff69b4.svg)](https://petdex.crafter.run)

> **✅ 跨平台支持：** 已通过 macOS 和 Windows 11 测试。

**Claude Code 桌宠插件** —— 一只实时展示你的 AI 编程助手工作状态的小精灵。支持 [Petdex](https://petdex.crafter.run) 社区 2700+ 精灵，一键切换。透明窗口、可拖拽、可缩放。

> 参考了 [Petdex](https://github.com/crafter-station/petdex) 的精灵格式和 HTML 模板设计。

## 效果预览

| 待机 | 工作中 | 放大 |
|------|--------|------|
| ![待机](idle.png) | ![工作中](working.png) | ![放大](zoomed.png) |

## 特性

- 🎨 **Petdex 兼容**：支持 [2700+ 社区精灵](https://petdex.crafter.run)，右键菜单直接搜索安装或随机
- 🔄 **实时状态同步**：根据 AI 助手状态自动切换动画（空闲/工作中/报错/确认）
- 🎯 **多会话支持**：多个 Agent 同时运行时取最高优先级状态
- 🖱️ **右键菜单**：切换宠物、搜索安装、随机一只、缩放、打开市场、退出
- 📐 **缩放**：0.5x ~ 1.5x 可调，窗口自动跟随，菜单全屏弹框
- 💬 **气泡**：Hook 状态气泡（持久）/ 闲时气泡（自动消失）/ 安装进度反馈
- 💤 **闲时动作**：30s 后概率触发互动，2 小时后休眠
- 🪟 **原生透明窗口**：macOS WKWebView / Windows WebView2，无黑底无闪烁
- 🌐 **内置宠物市场**：输入名字或随机，自动从 Petdex API 下载安装，无需 Node.js
- 📦 **轻量**：~3MB 二进制，无需额外运行时

## 快速开始

### Claude Code 插件安装（推荐）

1. 在 Claude Code 中输入 `/plugin`
2. 选择 **Marketplaces** → **Add Marketplace**
3. 输入 `Jedeiah/agent-critter`，回车
4. 回到 **Plugins**，找到 `agent-critter`，选择 **Install**
5. 输入 `/reload-plugins` 重载插件

安装后自动启动桌宠。

或命令行：
```
/plugin marketplace add github.com/Jedeiah/agent-critter
/plugin install agent-critter@agent-critter
/reload-plugins
```

### 独立运行

```bash
# 构建并运行
cargo run -- --daemon
```

> 也可直接下载 Release 的二进制，解压后运行 `bin/agent-critter --daemon`。

## 状态映射

| AI 状态 | 宠物动画 | 行 |
|---------|---------|----|
| 空闲 (Idle) | 呼吸待机 | 0 |
| 工作中 (Running) | 左右奔跑（随机切换） | 1/2 |
| 等待确认 (NeedConfirm) | 等待 | 6 |
| 工具异常 (ToolError) | 审查中 | 8 |
| 严重错误 (ErrorFinal) | 失败崩溃 | 5 |

## 架构

```
Claude Code Hooks → TCP(7890) → StateMachine → evaluate_script() → WebView
     (JSON)                      (多会话)         (瞬时推JS)         (CSS动画)
```

- **Rust daemon**：状态机 + TCP 服务器 + wry/tao 透明窗口
- **HTML/CSS/JS**：Petdex 格式精灵渲染（8×9 网格，JS 逐帧播）
- **WebView**：wry (macOS WKWebView / Windows WebView2)，透明 + 置顶 + 可拖拽

## 安装宠物精灵

在 [Petdex](https://petdex.crafter.run/zh/collections) 浏览 2700+ 免费精灵，选好后安装到本地。

### 方式一：右键菜单直接安装（推荐，无需 Node.js）

右键桌宠 → 打开菜单（分上下两部分，点击可放大）：

| 切换宠物 / 市场 / 安装 | 大小 / GitHub / 退出 |
|------------------------|---------------------|
| ![菜单1](菜单1.png) | ![菜单2](菜单2.png) |

- **📥装**：输入宠物名字后点击，自动从 Petdex API 下载并切换到该宠物
- **🎲随机**：从 2700+ 宠物中随机选一个下载安装
- **🌐 浏览市场找名字**：打开 Petdex 合集页面，浏览宠物图及其名字
- 安装完成后气泡会显示 **"✅ 已安装 xxx"** 并自动切换
- 如果已安装，气泡显示 **"✅ xxx 已存在"** 并直接切换过去
- 输入自动转小写、空格变 `-`，支持粘贴

> 名字支持 `slug`（如 `boba`）和 `displayName`（如 `Boba`），不区分大小写。

### 方式二：命令行（有 Node.js）

```bash
npx -y petdex install boba
```

替换 `boba` 为任意精灵名字即可。安装后右键菜单可见。

### 方式三：手动下载

1. 打开 [petdex.crafter.run/zh/collections](https://petdex.crafter.run/zh/collections)
2. 找到喜欢的精灵，点击下载 `spritesheet.webp`
3. 放入以下路径：

| 系统 | 路径 |
|------|------|
| macOS / Linux | `~/.codex/pets/<名字>/spritesheet.webp` |
| Windows | `%USERPROFILE%\.codex\pets\<名字>\spritesheet.webp` |

4. 右键菜单即可切换

### 已安装的宠物在哪

桌宠会扫描以下目录：

| 目录 | 说明 |
|------|------|
| `~/.codex/pets/` | Codex CLI 宠物目录（主目录） |
| `~/.petdex/pets/` | 旧版 Petdex 兼容目录 |

每个宠物一个文件夹，里面放 `spritesheet.webp`（或 `.png`）即可。

## 从源码构建

```bash
# 构建
cargo build --release

# 打包插件
bash scripts/build-plugin.sh
# Windows: build-plugin.bat
```

## 数据存储

配置保存在 `~/.agent-critter/data/`：
| 文件 | 内容 |
|------|------|
| `position` | 窗口位置（x, y） |
| `pet-scale` | 缩放比例（0.5 ~ 1.5） |

首次启动默认显示在屏幕右下角。

## 技术栈

| 层 | 技术 |
|----|------|
| 窗口 | wry + tao (macOS WKWebView / Windows WebView2) |
| 渲染 | CSS background-image + JS setTimeout 逐帧 |
| 状态机 | Rust StateMachine (多会话优先级) |
| Hook | TCP JSON (Claude Code plugin hooks) |
| 精灵 | Petdex 8×9 spritesheet (webp) |

## Roadmap

正在计划中的功能：

- [x] Claude Code 实时状态同步
- [x] 🌐 **精灵市场内置** — 右键菜单搜索安装 / 随机一只，无需 Node.js
- [ ] 适配更多 Agent — **Codex CLI** / **OpenCode** / **Gemini CLI**
- [ ] 更多 Hook 事件处理（Subagent、Compact 等）
- [ ] 🎙️ **宠物语音** — 状态切换时播放音效或 TTS 语音
- [ ] 主题系统 — 自定义 UI 配色

## 致谢

- [Petdex](https://github.com/crafter-station/petdex) — 精灵格式、HTML 模板参考
- [wry](https://github.com/nicehash/wry) — Rust WebView 库

## License

[MIT](LICENSE)
