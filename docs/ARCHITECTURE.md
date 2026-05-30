# Agent Critter — 架构文档

## 一、项目简介

一个**桌面浮动宠物**，连接 Claude Code 的 hook 系统，根据 Agent 工作状态自动切换宠物动画。支持 Petdex 社区的 2700+ 宠物精灵条，一键切换。

---

## 二、代码来源说明

| 组件 | 来源 | 改写程度 |
|------|------|---------|
| HTML/CSS/JS 模板 | Petdex `main.zig` 中的 `html_head` + `html_tail` | **直接复制**，只改了 spritesheet 注入方式（本地文件 → base64 内联）和加了 `setState`/`setBubble` 桥接函数 |
| STATES 动画定义 | Petdex JS `STATES` 对象 | **完全一致**，9 行 8 列，逐帧 timing |
| `play()` 逐帧动画引擎 | Petdex JS | **完全一致**，`setTimeout` 链 |
| Bubble 气泡系统 | Petdex JS `ensureBubble`/`positionBubble` | **完全一致** |
| 拖拽逻辑 | Petdex JS `mousedown/move/up` | **一致**，惯性已简化 |
| 精灵格式 | Petdex 8×9 网格 (`spritesheet.webp`) | **直接兼容** |
| Rust daemon (TCP + 状态机) | 我们自己写的 | — |
| Rust WebView 窗口 (wry/tao) | 我们自己写的 | — |
| macOS 透明窗口 | 参考 Petdex 的 `zero-native`（Zig）用 Rust + objc 重写 | 三步：`setOpaque:NO` + `clearColor` + `drawsBackground:NO` |

---

## 三、架构总览

```
┌─────────────────────────────────────────────────────┐
│  Claude Code                                        │
│  └─ hook 事件触发 → 调 agent-critter --hook         │
│                     (JSON via stdin)                 │
└────────────────────┬────────────────────────────────┘
                     │ TCP (port 7890)
                     ▼
┌─────────────────────────────────────────────────────┐
│  Rust Daemon 进程 (agent-critter --daemon)           │
│                                                      │
│  ┌─ server.rs ──────────────────────────────────┐   │
│  │  TCP server, 接收 HookPayload JSON           │   │
│  │  → map_hook_event() 映射为内部事件            │   │
│  │  → StateMachine.handle_event()               │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌─ state.rs ───────────────────────────────────┐   │
│  │  StateMachine                                │   │
│  │  ├─ HashMap<session_id, Session>  多会话管理  │   │
│  │  ├─ session_counter                会话计数    │   │
│  │  ├─ current_state       按优先级取最高状态    │   │
│  │  └─ should_exit         会话全结束 → 退出     │   │
│  │                                               │   │
│  │  LightState 优先级:                           │   │
│  │    1 = Idle                                   │   │
│  │    2 = Running                                │   │
│  │    3 = ToolError                              │   │
│  │    4 = ErrorFinal                             │   │
│  │    5 = NeedConfirm                            │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌─ daemon.rs ──────────────────────────────────┐   │
│  │  100ms polling: 读 StateMachine.              │   │
│  │  → proxy.send_event(UiCommand::SetState)      │   │
│  │  → EventLoop 处理 → evaluate_script() 推 JS   │   │
│  │                                               │   │
│  │  UiCommand 枚举:                              │   │
│  │    SetState {state, duration_ms}              │   │
│  │    Move {dx, dy}                              │   │
│  │    SwitchPet {slug}                           │   │
│  │    Quit                                       │   │
│  │                                               │   │
│  │  自动退出: 所有 session 结束后 2 秒退出       │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌─ webview.rs ────────────────────────────────┐   │
│  │  build_page(bytes, slug) → HTML String       │   │
│  │  find_first_pet() → spritesheet bytes        │   │
│  │  load_pet_bytes(slug) → spritesheet bytes    │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌─ WKWebView (wry) ───────────────────────────┐   │
│  │  窗口: 140×180, 透明, 无边框, 置顶          │   │
│  │  内容: Petdex HTML (精灵 + 气泡 + 拖拽)      │   │
│  │  JS 桥: setState() / setBubble()             │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

---

## 四、状态映射

| 内部 LightState | JS 状态名 | Petdex 精灵行 | 说明 |
|----------------|----------|--------------|------|
| Idle | `idle` | row 0 | 乖巧等待，6 帧呼吸循环 |
| Running | `running` | row 7 | 认真干活，6 帧工作动画 |
| NeedConfirm | `waving` | row 3 | 歪头疑惑，4 帧挥手 |
| ToolError | `review` | row 8 | 踩坑，6 帧审查 |
| ErrorFinal | `failed` | row 5 | 崩溃，8 帧失败 |

---

## 五、Hook 事件流

```
Claude Code 事件              →  内部事件       →  LightState
─────────────────────────────────────────────────────────────
SessionStart                  →  session_start  →  Idle
UserPromptSubmit / PreToolUse →  running        →  Running
PostToolUse                   →  running        →  Running
PermissionRequest             →  need_confirm   →  NeedConfirm
PostToolUseFailure (非中断)   →  tool_error     →  ToolError
StopFailure (auth/billing)    →  error_final    →  ErrorFinal
Stop                          →  stop           →  Idle (回退)
SessionEnd                    →  session_end    →  计数 -1
```

### 事件映射（hook.rs）

```rust
match hook_event_name {
    "SessionStart"      → (source=="compact")? "running" : "session_start"
    "UserPromptSubmit"  → "running"
    "PreToolUse"        → "running"
    "PostToolUse"       → "running"
    "PermissionRequest" → "need_confirm"
    "Notification"      → "need_confirm"  // permission_prompt / elicitation_dialog
    "Stop"              → "stop"
    "StopFailure"       → (auth/billing/model)? "error_final" : "tool_error"
    "PostToolUseFailure"→ (is_interrupt)? "stop" : "tool_error"
    "SessionEnd"        → "session_end"
}
```

---

## 六、宠物管理

### 发现机制

启动时扫描两个目录：
- `~/.codex/pets/<slug>/spritesheet.{webp,png}`
- `~/.petdex/pets/<slug>/spritesheet.{webp,png}`

取第一个找到的作为初始宠物。

### 切换

右键宠物 → 弹出 Petdex 风格的菜单 → 点击切换 → Rust 重载 HTML。

---

## 七、目录结构

```
agent-critter/
├── Cargo.toml              # wry, tao, base64, serde, rust-embed, objc(macOS)
├── src/
│   ├── main.rs             # 入口: --daemon / --hook / --event
│   ├── lib.rs             # 模块声明
│   ├── daemon.rs          # 主线程: WebView + EventLoop + 状态轮询
│   ├── server.rs          # TCP 服务器
│   ├── hook.rs            # HookPayload 解析 + 事件映射
│   ├── state.rs           # StateMachine (多会话状态机)
│   ├── webview.rs         # HTML 模板生成 (Petdex 复制)
│   ├── assets.rs          # rust-embed 默认精灵条
│   └── client.rs          # TCP 客户端 (--hook 模式用)
├── hooks/hooks.json        # Claude Code hooks 配置
└── assets/
    └── (可选) spritesheet.png  # 默认精灵，Petdex 8×9 格式
```

---

## 八、运行方式

```bash
# 1. 安装宠物 (社区 2700+ 可选)
npx -y petdex install boba

# 2. 启动桌面宠物
cargo run -- --daemon

# 3. 手动测试 (另一个终端)
echo '{"event":"running","session_id":"test"}' | nc 127.0.0.1 7890

# 4. 作为 Claude Code 插件安装
# hooks/hooks.json 已配置好，Claude Code 自动调 --hook
```

---

## 九、与 Petdex 的关系

| | Petdex | agent-critter |
|------|--------|------|
| 窗口技术 | Zig + zero-native + WKWebView | Rust + wry/tao + WKWebView |
| 精灵渲染 | CSS `background-image` + JS 逐帧 | **完全一致** |
| 状态驱动 | v0 只有 idle（roadmap 未实现） | ✅ 5 个状态，TCP 驱动 |
| 多会话 | ❌ | ✅ 多 session 优先级 |
| 交互 | 右键菜单, 设置窗口 | 右键宠物切换 |
| 社区精灵 | npx petdex install | ✅ 直接兼容 |
