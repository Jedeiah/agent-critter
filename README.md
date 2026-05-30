# Agent Critter 🐱

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Petdex](https://img.shields.io/badge/pets-2700%2B-ff69b4.svg)](https://petdex.crafter.run)

一只桌宠小精灵，实时展示你的 AI 编程助手（Claude Code / Codex / Gemini CLI）的工作状态。支持 [Petdex](https://petdex.crafter.run) 社区 2700+ 精灵，一键切换。透明窗口、可拖拽、可缩放。

> 参考了 [Petdex](https://github.com/crafter-station/petdex) 的精灵格式和 HTML 模板设计。

## 效果预览

```
   ┌──────────────┐
   │ （桌宠动画）  │  ← 干活时跑步，空闲时呼吸
   │  "收到！"    │  ← 状态气泡
   └──────────────┘
        ↑ 透明窗口，永远置顶，可拖拽移动
```

## 特性

- 🎨 **Petdex 兼容**：支持 [2700+ 社区精灵](https://petdex.crafter.run)，可上传图片自定义，一键 `npx petdex install`
- 🔄 **实时状态同步**：根据 AI 助手状态自动切换动画（空闲/工作中/报错/确认）
- 🎯 **多会话支持**：多个 Agent 同时运行时取最高优先级状态
- 🖱️ **交互**：单击互动、双击看状态、右键切换宠物+缩放
- 📐 **缩放**：0.5x ~ 1.5x 可调，窗口自动跟随
- 💬 **气泡**：Hook 状态气泡（持久）/ 闲时气泡（自动消失）
- 💤 **闲时动作**：30s 后概率触发互动，2 小时后休眠
- 🪟 **原生透明窗口**：wry + WKWebView，无黑底无闪烁
- 📦 **轻量**：~3MB 二进制，无需额外运行时

## 快速开始

### 作为 Claude Code 插件安装

1. 从 [Releases](https://github.com/Jedeiah/agent-critter/releases) 下载对应平台的插件包
2. 解压到本地目录
3. 在 Claude Code 中添加本地 marketplace：
   ```
   /plugin marketplace add /path/to/解压目录
   ```
4. 安装插件：
   ```
   /plugin install agent-critter@agent-critter
   ```

### 独立运行

```bash
# 下载桌面宠物精灵（可选，不装也有默认精灵）
npx -y petdex install boba

# 构建并运行
cargo run -- --daemon
```

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
Claude Code Hooks → TCP(7890) → StateMachine → evaluate_script() → WKWebView
     (JSON)                      (多会话)         (瞬时推JS)         (CSS动画)
```

- **Rust daemon**：状态机 + TCP 服务器 + wry/tao 透明窗口
- **HTML/CSS/JS**：Petdex 格式精灵渲染（8×9 网格，JS 逐帧播）
- **WebView**：wry (macOS WKWebView)，透明 + 置顶 + 可拖拽

## 精灵格式

兼容 Petdex 社区格式：单张 8 列 × 9 行 spritesheet（webp/png），每行一个动作。

```
安装: npx petdex install <name>  →  ~/.codex/pets/<name>/spritesheet.webp
使用: 右键 → 宠物列表切换
```

## 从源码构建

```bash
# 构建
cargo build --release

# 打包插件
bash scripts/build-plugin.sh
# Windows: build-plugin.bat
```

## 技术栈

| 层 | 技术 |
|----|------|
| 窗口 | wry + tao (Rust, WKWebView) |
| 渲染 | CSS background-image + JS setTimeout 逐帧 |
| 状态机 | Rust StateMachine (多会话优先级) |
| Hook | TCP JSON (Claude Code plugin hooks) |
| 精灵 | Petdex 8×9 spritesheet (webp) |

## Roadmap

正在计划中的功能：

- [x] Claude Code 实时状态同步
- [ ] 适配更多 Agent — **Codex CLI** / **OpenCode** / **Gemini CLI**
- [ ] 更多 Hook 事件处理（Subagent、Compact 等）
- [ ] 🎙️ **宠物语音** — 状态切换时播放音效或 TTS 语音
- [ ] 精灵市场内置（直接浏览和安装 Petdex 社区精灵）
- [ ] 主题系统 — 自定义 UI 配色

## 致谢

- [Petdex](https://github.com/crafter-station/petdex) — 精灵格式、HTML 模板参考
- [wry](https://github.com/nicehash/wry) — Rust WebView 库

## License

MIT © [chj](https://github.com/Jedeiah)
