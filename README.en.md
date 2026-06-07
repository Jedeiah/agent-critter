# Agent Critter 🐱

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Petdex](https://img.shields.io/badge/pets-2700%2B-ff69b4.svg)](https://petdex.dev)
[![Plugin Hub](https://img.shields.io/badge/plugin-available-8A2BE2.svg)](https://www.claudepluginhub.com/plugins/jedeiah-agent-critter)

> **English · [中文](README.md) 🌏**

> **Cross-platform:** Tested on macOS and Windows 11.

**Agent Critter** is a Claude Code desktop pet plugin — a cute critter that reflects your AI coding assistant's working status in real time. Supports 2700+ pets from the [Petdex](https://petdex.dev) community with one-click switching. Transparent window, draggable, resizable.

---

## Preview

| Idle | Working | Zoomed |
|------|---------|--------|
| ![Idle](idle.png) | ![Working](working.png) | ![Zoomed](zoomed.png) |

---

## Installation

### Prerequisites

- [Claude Code](https://code.claude.com) CLI (plugin mode)
- Or: macOS / Windows system (standalone mode)

### Install via Plugin Marketplace (Recommended)

```bash
# Add marketplace
/plugin marketplace add github.com/Jedeiah/agent-critter

# Install plugin
/plugin install agent-critter@agent-critter

# Reload plugins
/reload-plugins
```

Or through the interactive menu:

1. Type `/plugin` in Claude Code
2. Select **Marketplaces** → **Add Marketplace**
3. Enter `Jedeiah/agent-critter`
4. Go back to **Plugins**, find `agent-critter`, select **Install**
5. Type `/reload-plugins`

The pet starts automatically after installation.

### Install from Release

Download the plugin package for your platform from [Releases](https://github.com/Jedeiah/agent-critter/releases), extract and run:

```bash
# macOS
./bin/agent-critter --daemon

# Windows
bin\agent-critter.exe --daemon
```

### Build from Source

```bash
cargo build --release

# Package plugin (optional)
bash scripts/build-plugin.sh   # macOS / Linux
# or
.\scripts\build-plugin.bat     # Windows
```

---

## Usage

The pet appears as a transparent floating window on top of all other windows, reflecting Claude Code's status in real time.

### Interaction

| Action | Effect |
|--------|--------|
| **Single-click** | Random interaction text + animation (waving / jumping / waiting / review) |
| **Double-click** | Show current session count and status bubble |
| **Drag** | Move the pet (drag the background area) |
| **Right-click** | Open the menu |

### Right-click Menu

| Pet Switching / Market / Install | Size / GitHub / Quit |
|----------------------------------|---------------------|
| ![Menu 1](菜单1.png) | ![Menu 2](菜单2.png) |

Right-click the pet to open the fullscreen menu:

```
🐾 Switch Pet
  ├─ Boba / Dwight / ...    ← Installed pets, click to switch
  ├──────────────────────
  🔍 Size x1.0  [−]  [+]   ← Scale from 0.5x to 1.5x
  ├──────────────────────
  🌐 Browse Market          ← Open Petdex collection page
  📥 [input] Install        ← Enter name to install a new pet
  🎲 Random                 ← Install a random pet
  ├──────────────────────
  ⭐ Star on GitHub         ← Open project homepage
  × Quit                    ← Close the pet
```

### Install More Pets

Built-in pet market — no Node.js required:

```bash
# Enter a name in the right-click menu, or click "Random"
# You can also use the CLI (requires Node.js):
npx -y petdex install <pet-name>
```

Browse 2700+ pets from the Petdex community: [https://petdex.dev/collections](https://petdex.dev/collections)

### State Mapping

Claude Code status changes automatically switch the pet's animation:

| AI State | Pet Animation | Description |
|----------|---------------|-------------|
| Idle | 😴 Breathing | Random idle interactions after 30s, auto-sleep after 2h |
| Running | 🏃 Running left/right | When processing user requests or executing tools |
| Need Confirm | ⏳ Waiting | Permission requests, confirmation dialogs |
| Tool Error | 🔍 Reviewing | Tool call failures, rate limiting |
| Error Final | 💥 Crashed | Auth failures, billing errors, model not found |

---

## Hook Events

Claude Code drives the pet's status through these hook events:

| Hook Event | Mapped State | Trigger |
|------------|--------------|---------|
| `SessionStart` | session_start | Session start / compact complete |
| `PerCompact` | running | Compact begins |
| `UserPromptSubmit` | running | User submits a prompt |
| `PreToolUse` | running | Before tool use |
| `PostToolUse` | running | After tool use |
| `PermissionRequest` | need_confirm | Permission request |
| `Notification` | idle / need_confirm | Notification |
| `Stop` | idle | Stop |
| `StopFailure` | tool_error / error_final | Stop failure |
| `PostToolUseFailure` | tool_error / stop | Tool use failure |
| `SessionEnd` | session_end | Session ends |

See [`hooks/hooks.json`](hooks/hooks.json) for all hook configurations.

---

## Architecture

```
Claude Code Hooks ──TCP(7890)──▶ StateMachine ──evaluate_script()──▶ WebView
     (JSON)                     (multi-session)    (instant JS push)    (CSS anim)
```

| Layer | Technology |
|-------|------------|
| Window | wry + tao (macOS WKWebView / Windows WebView2) |
| Rendering | CSS background-image + JS setTimeout frame loop |
| State Machine | Rust StateMachine (multi-session priority) |
| Hook | TCP JSON (Claude Code plugin hooks) |
| Sprite | Petdex 8×9 spritesheet (webp) |

### Multi-session Priority

When multiple Claude Code sessions run simultaneously, the pet shows the highest priority status:

`ErrorFinal > ToolError > NeedConfirm > Running > Idle`

---

## Configuration

All settings are persisted in `~/.agent-critter/data/`:

| File | Content | Description |
|------|---------|-------------|
| `position` | `x\ny` | Window position (pixel coordinates) |
| `pet-slug` | `boba` | Currently selected pet slug |
| `pet-scale` | `1.0` | Scale factor (0.5 ~ 1.5) |

On first launch, the pet appears at the bottom-right corner of the screen.

---

## Pet Storage

The pet scans these directories for installed pets:

| Directory | Description |
|-----------|-------------|
| `~/.codex/pets/<name>/` | Main directory (auto-created) |
| `~/.petdex/pets/<name>/` | Legacy Petdex compatibility |

Each pet has its own folder containing `spritesheet.webp` (or `.png`).

---

## Uninstall

```bash
/plugin uninstall agent-critter
```

To fully remove all data:

```bash
rm -rf ~/.agent-critter
```

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| **Pet not visible** | Run `pkill agent-critter` and restart Claude Code |
| **Port 7890 in use** | Run `pkill agent-critter` or change the port |
| **Pet image not showing** | Check `~/.codex/pets/<name>/spritesheet.webp` exists |
| **Windows white border** | Handled automatically; file an issue if it persists |

---

## Roadmap

- [x] Real-time Claude Code status sync
- [x] Built-in pet market (search install / random)
- [ ] Support more agents (Codex CLI / OpenCode / Gemini CLI)
- [ ] More hook events (Subagent, Compact, etc.)
- [ ] Pet voice (sound effects / TTS on status change)
- [ ] Theme system (custom UI colors)

---

## Credits

- [Petdex](https://github.com/crafter-station/petdex) — Sprite format and HTML template reference
- [wry](https://github.com/nicehash/wry) — Rust WebView library
- [tao](https://github.com/tauri-apps/tao) — Cross-platform windowing

## License

[MIT](LICENSE)
