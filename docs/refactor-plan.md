# Agent Critter 重构方案

## 目标

用 `wry` (Rust WebView) 替代 `FLTK`，实现 Petdex 级别的透明窗口 + CSS 逐帧动画，
保留我们的多会话状态机、面板交互、主题切换。

---

## 一、技术栈对比

| | 旧（FLTK） | 新（wry） |
|------|-----------|----------|
| 窗口 | FLTK DoubleWindow | NSWindow + WKWebView |
| 透明 | ❌ macOS 黑底 bug | ✅ 原生透明 |
| 精灵渲染 | `draw_image` 逐帧 blit | CSS `steps()` GPU 加速 |
| 动画帧率 | 33ms Rust loop | 浏览器原生 requestAnimationFrame |
| 交互 | FLTK Event handler | HTML/CSS/JS |
| 面板 | 第二个 FLTK Window | HTML 内 `<div>` |
| 主题切换 | Rust draw 函数 | JS 换 spritesheet URL |
| 状态通信 | Mutex 共享 → idle loop redraw | `evaluate_script()` 直推 JS |
| 体积 | ~2MB | ~3MB (wry 比 FLTK 略大) |
| 跨平台 | Windows/macOS/Linux | Windows/macOS/Linux |

---

## 二、精灵格式：从竖排改 8×9 网格

### 旧格式（5 个文件，竖排）

```
idle.png         256×7680  (256×256 × 30帧)
running.png      256×7680
need_confirm.png 256×7680
tool_error.png   256×7680
error_final.png  256×7680
```

### 新格式（1 个文件，8×9 网格）

```
spritesheet.webp  1728×1664  (192×208 × 8行 × 9列)

列 0-5: 该状态的动画帧（6帧循环）
列 6-8: 预留

行 0: idle         → LightState::Idle
行 1: waving       → LightState::NeedConfirm
行 2: running      → LightState::Running
行 3: failed       → LightState::ErrorFinal
行 4: review       → LightState::ToolError
行 5: jumping      → 预留（可作错误抖动/弹跳）
行 6: extra1       → 预留
行 7: extra2       → 预留
```

### 为什么要换

- **兼容 Petdex 社区**：`npx petdex install boba` 的精灵直接拖进来用
- **1 文件 vs 5 文件**：更整洁
- **CSS `steps()`** 天然适合网格精灵条
- **GPU 加速**：浏览器原生处理，不卡

---

## 三、架构总览

```
┌────────────────────────────────────────────────────┐
│  NSWindow                                          │
│  ├─ frameless: true                                │
│  ├─ transparent: true                              │
│  ├─ level: NSFloatingWindowLevel                   │
│  ├─ collectionBehavior: canJoinAllSpaces           │
│  │                                                  │
│  └─ WKWebView (wry)                                │
│      ├─ HTML (内联 spritesheet + CSS + JS)          │
│      │                                              │
│      │  CSS: steps() animation                     │
│      │  JS:  setState() / setBubble() / drag        │
│      │       setTheme() / togglePanel()             │
│      │                                              │
│      └─ 事件流 ← evaluate_script()                 │
│                    ↑                                │
│              Rust daemon                            │
│              ├─ TCP server (port 7890)              │
│              ├─ StateQueue (250ms debounce)         │
│              ├─ StateMachine (多会话优先级)          │
│              ├─ ThemeManager                        │
│              └─ WebView bridge                      │
│                    ↑ TCP                            │
│              Claude Code hooks                      │
└────────────────────────────────────────────────────┘
```

---

## 四、Rust 侧架构

### 目录结构

```
src/
├── main.rs          # 入口：解析参数，决定 daemon 或 hook 模式
├── daemon.rs        # 启动 TCP server + webview
├── server.rs        # TCP 监听，解析 hook JSON
├── hook.rs          # 事件映射（不变）
├── state.rs         # StateMachine + StateQueue（新增）
├── theme.rs         # 主题管理、精灵路径
├── webview.rs       # wry 窗口创建 + JS 桥
└── lib.rs
```

### 模块职责

| 模块 | 职责 | 变化 |
|------|------|------|
| `main.rs` | 入口 + 参数解析 | 不变 |
| `daemon.rs` | 启动 WebView + server 线程 | 重写（去掉 FLTK） |
| `server.rs` | TCP 监听 + HookPayload 解析 | 不变 |
| `hook.rs` | `map_hook_event()` | 不变 |
| `state.rs` | StateQueue + StateMachine | 新增 StateQueue |
| `theme.rs` | 精灵路径、主题选择 | 简化（不再需要 SpriteStrip） |
| `webview.rs` | wry 窗口、HTML 生成、JS 桥 | **新增** |

### state.rs 设计

```rust
/// StateQueue: 防抖 + duration 自动回落
pub struct StateQueue {
    queue: VecDeque<QueuedState>,
    current_state: LightState,
    last_emit: Instant,
    min_dwell: Duration,     // 250ms
    pending_duration: Option<Duration>, // duration 回落计时
}

struct QueuedState {
    state: LightState,
    duration: Option<Duration>,
    received_at: Instant,
}

impl StateQueue {
    /// Hook 事件入口
    pub fn enqueue(&mut self, state: LightState, duration: Option<Duration>);

    /// 每 100ms tick，返回是否需要切换
    pub fn tick(&mut self, now: Instant) -> Option<LightState>;
}
```

逻辑：
1. 所有 hook 事件进队
2. tick 时取队尾（最新）状态
3. 如果该状态已停留 ≥250ms → 切换
4. 如果有 duration → 开始倒计时，到时间自动回 idle
5. 队列长度上限 50，超出丢最旧的

### webview.rs 设计

```rust
pub struct PetWebView {
    webview: wry::WebView,
    window: wry::application::window::Window,
}

impl PetWebView {
    pub fn new(theme_slug: &str) -> Self;

    /// JS 桥：切换动画状态
    pub fn set_state(&self, state: &str, duration_ms: Option<u64>);

    /// JS 桥：显示气泡
    pub fn set_bubble(&self, text: &str);

    /// JS 桥：切换主题（换 spritesheet）
    pub fn set_theme(&self, slug: &str);

    /// JS 桥：同步面板数据（会话数、最近事件）
    pub fn set_panel(&self, sessions: u32, event: &str, state: &str);
}
```

---

## 五、HTML/CSS/JS 设计

### HTML 结构

```html
<div id="stage" style="-webkit-app-region: drag">
  <!-- 主窗口拖拽区 -->

  <div id="pet" class="idle">
    <!-- CSS steps() 动画 -->
  </div>

  <div id="bubble" class="hidden">
    <!-- 状态气泡文字 -->
  </div>

  <div id="panel" class="hidden">
    <!-- 状态面板 / 主题选择器 -->
    <div id="panel-status">
      <div class="status-dot"></div>
      <span id="state-label">乖巧等待</span>
      <span id="session-count">活跃: 1</span>
      <span id="recent-event">最近: —</span>
      <button id="exit-btn">× 退出</button>
    </div>
    <div id="panel-theme" class="hidden">
      <!-- 主题列表，同 draw_theme_chooser -->
    </div>
  </div>
</div>
```

### CSS

```css
/* 精灵尺寸 */
:root {
  --fw: 192px;   /* 帧宽 */
  --fh: 208px;   /* 帧高 */
  --cols: 6;     /* 动画列数 */
}

/* 窗口：透明 */
html, body, #stage {
  margin: 0; padding: 0;
  width: 256px; height: 256px;
  background: transparent;
  overflow: hidden;
}

/* 精灵：背景图 + CSS steps */
#pet {
  width: var(--fw);
  height: var(--fh);
  background-image: url('data:image/webp;base64,...');
  background-size: 1728px 1664px;  /* 9*192, 8*208 */
  background-repeat: no-repeat;
  margin: 24px auto 0;  /* (256-208)/2 = 24 */
}

/* 每一行的 Y 偏移 */
#pet.idle         { background-position-y: 0;      }
#pet.running      { background-position-y: -208px; }
#pet.waving       { background-position-y: -416px; }
#pet.failed       { background-position-y: -624px; }
#pet.review       { background-position-y: -832px; }
#pet.jumping      { background-position-y: -1040px; }

/* 逐帧动画 */
#pet.animating {
  animation: walk 1.1s steps(6) infinite;
}
@keyframes walk {
  from { background-position-x: 0; }
  to   { background-position-x: calc(-1 * var(--fw) * var(--cols)); }
}

/* 气泡 */
#bubble {
  position: absolute; top: 4px; left: 50%;
  transform: translateX(-50%);
  background: rgba(30,35,55,0.9);
  color: #f0f4ff;
  padding: 4px 12px; border-radius: 6px;
  font-size: 11px; font-family: sans-serif;
  transition: opacity 0.3s;
}
#bubble.hidden { opacity: 0; pointer-events: none; }

/* 面板 */
#panel { /* ... */ }
```

### JS 桥函数

```javascript
// Rust 调用的入口
function setState(state, durationMs) {
  pet.className = state;
  pet.classList.add('animating');
  if (durationMs) {
    setTimeout(() => setState('idle', 0), durationMs);
  }
}

function setBubble(text) {
  if (!text) { bubble.classList.add('hidden'); return; }
  bubble.textContent = text;
  bubble.classList.remove('hidden');
  setTimeout(() => bubble.classList.add('hidden'), 2500);
}

function setTheme(slug) {
  // fetch spritesheet and update CSS background-image
}

function setPanel(sessions, event, stateLabel) {
  // update panel status display
}
```

---

## 六、交互设计

### 左键点击

```
单击角色 → JS 通知 Rust → togglePanel()
右键角色 → toggleThemePanel()
拖拽角色 → -webkit-app-region: drag（无需 JS）
```

### 面板交互

与现在一致：
- 状态面板：显示会话数、最近事件、退出按钮
- 主题面板：点击切换主题
- 左键/右键在角色上 → 切换面板

### 实现方式

```
JS 端：
  监听 click / contextmenu 事件
  → 调 Rust 的 ipc handler（不需要，直接在 JS 里处理面板）

面板在 HTML 内，完全 JS 控制，不需要 Rust 参与。
退出按钮 → JS 调 `window.ipc.postMessage('quit')`
```

---

## 七、依赖变化

### Cargo.toml

```toml
# 删掉
fltk = ...
image = ...      # 不再需要解码 PNG 逐帧
rust-embed = "8" # 可选保留（嵌入 HTML），或用 include_str!

# 新增
wry = "0.50"     # WebView
tao = "0.32"     # 窗口创建（wry 依赖）

# 保留
serde = "1.0"
serde_json = "1.0"

# 删掉（仅 Windows）
windows-sys = ...  # wry 自带窗口管理

# 删掉（仅 macOS）
objc = ...         # wry 自带

# 保留
rust-embed = "8"   # 嵌入 HTML 模板 + spritesheet
```

### 精灵嵌入

```rust
#[derive(RustEmbed)]
#[folder = "assets/"]
struct AppAssets;
// assets/spritesheet.webp   → 默认精灵
// assets/index.html         → HTML 模板
```

---

## 八、实施步骤

### Step 1：搭架子（wry 窗口 + HTML）

- 删 FLTK 依赖，加 wry/tao
- `webview.rs`：创建透明窗口、加载内联 HTML
- `daemon.rs`：启动 WebView + TCP server 线程
- 验证：窗口出现，透明，能看到精灵

### Step 2：JS 桥 + 状态机

- `setState()` JS 函数
- `StateQueue` + tick
- `evaluate_script()` 推状态
- 验证：hook 事件 → 精灵切换动画

### Step 3：面板 + 交互

- HTML 面板（状态 + 主题）
- 点击、右键处理
- 退出按钮
- 验证：交互正常

### Step 4：社区宠物兼容

- 从 `~/.petdex/pets/<slug>/` 读 spritesheet
- `npx petdex install boba` → 直接选
- 验证：切换社区宠物

---

## 九、从 Petdex 直接借鉴的实现细节

### 9.1 JS 动画：不用 CSS keyframes，用 JS 手动切帧

Petdex 不用 CSS `@keyframes steps()`。它用 JS 的 `setTimeout` 链：

```javascript
const STATES = {
  idle: { row: 0, frames: [
    {c:0,d:280}, {c:1,d:110}, {c:2,d:110},
    {c:3,d:140}, {c:4,d:140}, {c:5,d:320}
  ]},
  running: { row: 7, count: 6, dur: 120, last: 220 },
  failed:  { row: 5, count: 8, dur: 140, last: 240 },
  // ...
};

function play(state) {
  const def = STATES[state];
  const frames = buildFrames(def);
  let i = 0;
  pet.style.backgroundPosition = pos(frames[0].c, frames[0].r);
  if (frames.length === 1) return;
  const tick = () => {
    stateTimer = setTimeout(() => {
      i = (i + 1) % frames.length;
      pet.style.backgroundPosition = pos(frames[i].c, frames[i].r);
      tick();
    }, frames[i].d);
  };
  tick();
}
```

**好处**：每帧可以有不同的持续时间——idle 的慢呼吸帧 280ms，快眨眼帧 110ms。CSS steps() 做不到。

### 9.2 精灵定位

```javascript
function pos(c, r) {
  // 8列 9行，百分比定位
  return `${c/(COLS-1)*100}% ${r/(ROWS-1)*100}%`;
}
```

`background-size: 800% 900%`（8列 × 9行 × 100%），然后 `background-position: x% y%` 定位到具体帧。

### 9.3 气泡定位

```javascript
function positionBubble() {
  const rect = pet.getBoundingClientRect();
  const petCenterX = rect.left + rect.width / 2;
  bubbleEl.style.left = (petCenterX - bw/2) + 'px';
  bubbleEl.style.top  = (rect.top - bh - 14) + 'px';
}
```

气泡始终在角色上方，用 `getBoundingClientRect()` 自动跟踪。

### 9.4 拖拽实现

```javascript
// mousedown → 记录起始位置
// mousemove → 更新 pet 位置
// mouseup   → 惯性 throw（momentum deceleration）
```

不依赖 `-webkit-app-region: drag`，而是 JS 手动处理，能做惯性甩出效果。

### 9.5 面板/菜单

```css
.menu {
  position: fixed;
  background: rgba(20,20,22,0.96);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 8px;
  backdrop-filter: blur(16px);
  z-index: 999;
}
```

毛玻璃菜单，grid 布局显示宠物列表。我们改造成状态面板 + 主题选择。

### 9.6 窗口尺寸

```zig
const WINDOW_W: f32 = 140;
const WINDOW_H: f32 = 180;
```

精灵 192×208 缩放到 4.5rem（~72px 在 1x 屏幕上）。我们的精灵用 256×256，窗口设为 256×256 即可。

---

## 十、风险与边界

| 风险 | 缓解 |
|------|------|
| wry/tao API 不够稳定 | 锁版本，或用 Tauri 替代（但体积大） |
| WebView 内存占用 | 实测：一个 256×256 空 WebView ~30MB，可接受 |
| CSS steps 帧率 VS 原生 | GPU 加速 + requestAnimationFrame，够用 |
| Windows 透明窗口 | wry 支持，但可能需要额外配置 |
| 精灵加载延迟 | base64 内联或 `rust-embed` 编译时嵌入，零延迟 |
