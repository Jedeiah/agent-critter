use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use crate::server::start_server;
use crate::state::LightState;
use crate::state::StateMachine;

const FIXED_PORT: u16 = 7890;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PetInfo {
    pub slug: String,
    pub name: String,
    #[serde(skip)]
    pub spritesheet_path: String,
}

fn list_pets() -> Vec<PetInfo> {
    let mut pets = Vec::new();
    let home = match crate::home_dir() { Some(h) => h, None => return pets };
    let home = std::path::PathBuf::from(home);
    let bases = [home.join(".codex").join("pets"), home.join(".petdex").join("pets")];
    for base in &bases {
        let dir = match std::fs::read_dir(base) { Ok(d) => d, Err(_) => continue };
        for entry in dir.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let slug = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            if slug.is_empty() || slug.contains("..") { continue; }
            for ext in &["webp", "png"] {
                let sheet = path.join(format!("spritesheet.{}", ext));
                if sheet.exists() {
                    pets.push(PetInfo { name: slug.clone(), slug, spritesheet_path: sheet.to_string_lossy().to_string() });
                    break;
                }
            }
        }
    }
    pets.sort_by(|a, b| a.slug.cmp(&b.slug));
    pets
}

#[derive(Debug, Clone)]
pub enum UiCommand {
    SetState { state: LightState, duration_ms: Option<u64> },
    SwitchRunning,
    SwitchPet { slug: String },
    IdleAction { action_state: &'static str, bubble: &'static str },
    SessionCount(u32),
    Quit,
}

fn state_name(s: LightState) -> &'static str {
    match s {
        LightState::Idle => "idle",
        LightState::Running => "running-right", // default
        LightState::NeedConfirm => "waiting",
        LightState::ToolError => "review",
        LightState::ErrorFinal => "failed",
    }
}

fn bubble_text(s: LightState) -> &'static str {
    match s {
        LightState::Running => "收到！", LightState::NeedConfirm => "等等...",
        LightState::ToolError => "哎呀！", LightState::ErrorFinal => "救我！",
        LightState::Idle => "",
    }
}

pub fn run_daemon(port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .map_err(|_| format!("Port {} already in use", port))?;
    let state = Arc::new(Mutex::new(StateMachine::new()));
    let mut event_loop = EventLoopBuilder::<UiCommand>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let state_srv = Arc::clone(&state);
    std::thread::spawn(move || { start_server(listener, state_srv); });

    // Window
    let mut window_builder = WindowBuilder::new()
        .with_inner_size(tao::dpi::LogicalSize::new(140.0, 180.0))
        .with_decorations(false).with_transparent(true).with_always_on_top(true)
        .with_resizable(false);
    #[cfg(target_os = "windows")]
    { use tao::platform::windows::WindowBuilderExtWindows;
      window_builder = window_builder.with_skip_taskbar(true).with_no_redirection_bitmap(true); }
    let window = window_builder.build(&event_loop).expect("window");

    // Windows: 移除标题栏 + 白边
    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowExtWindows;
        unsafe {
            extern "system" {
                fn SetWindowLongPtrW(hwnd: isize, index: i32, new_long: isize) -> isize;
                fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
                fn SetWindowPos(hwnd: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
            }
            let hwnd = window.hwnd();
            let style = GetWindowLongPtrW(hwnd, -16);
            SetWindowLongPtrW(hwnd, -16, style & !((0x00800000_i32 | 0x00040000 | 0x00C00000) as isize));
            let ex_style = GetWindowLongPtrW(hwnd, -20);
            SetWindowLongPtrW(hwnd, -20, ex_style & !((0x00000001_i32 | 0x00000200 | 0x00020000) as isize));
            SetWindowPos(hwnd, 0, 0, 0, 0, 0, 0x0020 | 0x0002 | 0x0001 | 0x0004);
        }
    }

    // Restore saved position, or default to bottom-right
    if let Some((x, y)) = load_position() {
        window.set_outer_position(tao::dpi::PhysicalPosition::new(x, y));
    } else if let Some(monitor) = window.primary_monitor() {
        let screen = monitor.size();
        let win_size = window.outer_size();
        window.set_outer_position(tao::dpi::PhysicalPosition::new(
            (screen.width as i32).saturating_sub(win_size.width as i32 + 40),
            (screen.height as i32).saturating_sub(win_size.height as i32 + 40),
        ));
    }

    #[cfg(target_os = "macos")]
    { use tao::platform::macos::EventLoopExtMacOS;
      event_loop.set_activation_policy(tao::platform::macos::ActivationPolicy::Accessory); }

    // Wrap window for sharing with IPC handler (Petdex-style direct drag)
    let win_arc = Arc::new(Mutex::new(window));
    let window = win_arc.clone(); // keep name `window` for later

    // Load pet: try saved slug first, fallback to first found, then default
    let (sheet, slug) = load_pet_slug()
        .and_then(|s| crate::webview::load_pet_bytes(&s).map(|b| (b, s)))
        .unwrap_or_else(|| crate::webview::find_first_pet()
            .unwrap_or_else(|| (include_bytes!("../assets/default_spritesheet.webp").to_vec(), "default".into())));
    let pets = list_pets();
    let pets_json = serde_json::to_string(&pets).unwrap_or_default();
    let saved_scale: f64 = std::fs::read_to_string(data_dir().join("pet-scale"))
        .ok().and_then(|s| s.trim().parse().ok()).unwrap_or(1.0);
    let html = crate::webview::build_page(&slug, &pets_json, saved_scale);

    // Custom protocol: / 返回 HTML，/sprite 返回精灵图
    let html_bytes: Vec<u8> = html.into_bytes();
    let sprite_data: Arc<Mutex<(Vec<u8>, &'static str)>> = Arc::new(Mutex::new(
        if sheet.len() >= 12 && &sheet[0..4] == b"RIFF" && &sheet[8..12] == b"WEBP" {
            (sheet, "image/webp")
        } else {
            (sheet, "image/png")
        }
    ));
    let sprite_serve = Arc::clone(&sprite_data);

    // WebView: IPC handler moves window directly, no EventLoop roundtrip
    let proxy_ipc = proxy.clone();
    let drag_win = win_arc.clone();

    let webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_custom_protocol("pet".into(), move |_id, request| {
            let path = request.uri().path();
            if path == "/sprite" {
                let guard = sprite_serve.lock().unwrap();
                let mime = guard.1;
                let bytes = guard.0.clone();
                wry::http::Response::builder()
                    .header("Content-Type", mime)
                    .header("Cache-Control", "no-store")
                    .body(std::borrow::Cow::from(bytes))
                    .unwrap()
            } else {
                wry::http::Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(std::borrow::Cow::from(html_bytes.clone()))
                    .unwrap()
            }
        })
        .with_url("pet://localhost/")
        .with_ipc_handler(move |msg| {
            if msg.body() == "quit" { std::process::exit(0); }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(msg.body()) {
                if let Some(slug) = v.get("theme").and_then(|t| t.as_str()) {
                    let _ = proxy_ipc.send_event(UiCommand::SwitchPet { slug: slug.into() });
                }
                if let Some(url) = v.get("url").and_then(|u| u.as_str()) {
                    if url == "https://github.com/Jedeiah/agent-critter" {
                        #[cfg(target_os = "macos")]
                        { let _ = std::process::Command::new("open").arg(url).spawn(); }
                        #[cfg(target_os = "windows")]
                        { let _ = std::process::Command::new("cmd").args(["/c", "start", url]).spawn(); }
                    }
                }
                if v.get("type").and_then(|t| t.as_str()) == Some("savePos") {
                    if let Ok(w) = drag_win.lock() {
                        let pos = w.outer_position().unwrap_or_default();
                        save_position(pos.x, pos.y);
                    }
                }
                if v.get("act").and_then(|a| a.as_str()) == Some("1") {
                    #[cfg(target_os = "macos")]
                    if let Ok(w) = drag_win.lock() {
                        use tao::platform::macos::WindowExtMacOS;
                        let ns = w.ns_window();
                        if !ns.is_null() {
                            unsafe {
                                let ns_obj = ns as *mut objc::runtime::Object;
                                let cls: *mut objc::runtime::Object = objc::msg_send![ns_obj, class];
                                if !cls.is_null() {
                                    let app: *mut objc::runtime::Object = objc::msg_send![objc::class!(NSApplication), sharedApplication];
                                    let _: () = objc::msg_send![app, activateIgnoringOtherApps: true];
                                    let _: () = objc::msg_send![ns_obj, makeKeyAndOrderFront: std::ptr::null::<objc::runtime::Object>()];
                                }
                            }
                        }
                    }
                }
                if v.get("type").and_then(|t| t.as_str()) == Some("saveScale") {
                    if let Some(s) = v.get("scale").and_then(|s| s.as_f64()) {
                        save_scale(s.clamp(0.5, 1.5) as f32);
                    }
                }
                if v.get("type").and_then(|t| t.as_str()) == Some("resize") {
                    let w = v.get("w").and_then(|d| d.as_u64()).unwrap_or(140).clamp(80, 600) as u32;
                    let h = v.get("h").and_then(|d| d.as_u64()).unwrap_or(180).clamp(80, 600) as u32;
                    if let Ok(win) = drag_win.lock() {
                        let _ = win.set_inner_size(tao::dpi::LogicalSize::new(w as f64, h as f64));
                    }
                }
                if v.get("type").and_then(|t| t.as_str()) == Some("dragStart") {
                    if let Ok(w) = drag_win.lock() {
                        let pos = w.outer_position().unwrap_or_default();
                        *drag_origin_ipc.lock().unwrap() = (pos.x, pos.y);
                    }
                }
                if v.get("type").and_then(|t| t.as_str()) == Some("move") {
                    let dx = v.get("dx").and_then(|d| d.as_i64()).unwrap_or(0) as i32;
                    let dy = v.get("dy").and_then(|d| d.as_i64()).unwrap_or(0) as i32;
                    if let Ok(w) = drag_win.lock() {
                        let pos = w.outer_position().unwrap_or_default();
                        w.set_outer_position(tao::dpi::PhysicalPosition::new(pos.x + dx, pos.y + dy));
                    }
                }
            }
        })
        .build(&*window.lock().unwrap()).expect("webview");

    // macOS: WKWebView drawsBackground = NO (must be AFTER webview build)
    #[cfg(target_os = "macos")]
    { use tao::platform::macos::WindowExtMacOS;
      let w = window.lock().unwrap();
      let ns: *mut std::ffi::c_void = w.ns_window();
      if !ns.is_null() { unsafe {
        let win: &objc::runtime::Object = &*(ns as *const objc::runtime::Object);
        let _: () = objc::msg_send![win, setOpaque: false];
        let _: () = objc::msg_send![win, setAcceptsMouseMovedEvents: true];
        let _: () = objc::msg_send![win, setMovableByWindowBackground: true];
        let c: *mut objc::runtime::Object = objc::msg_send![objc::class!(NSColor), clearColor];
        let _: () = objc::msg_send![win, setBackgroundColor: c];
        let cv: *mut objc::runtime::Object = objc::msg_send![win, contentView];
        if !cv.is_null() {
          let subs: *mut objc::runtime::Object = objc::msg_send![cv, subviews];
          if !subs.is_null() {
          let n: usize = objc::msg_send![subs, count];
          for i in 0..n {
            let v: *mut objc::runtime::Object = objc::msg_send![subs, objectAtIndex: i];
            if v.is_null() { continue; }
            let no: *mut objc::runtime::Object = objc::msg_send![objc::class!(NSNumber), numberWithBool: false];
            let k: *mut objc::runtime::Object = objc::msg_send![objc::class!(NSString), stringWithUTF8String: b"drawsBackground\0".as_ptr() as *const i8];
            let _: () = objc::msg_send![v, setValue: no forKey: k];
          }
          }
        }
      }}
    }

    { let w = window.lock().unwrap(); w.set_visible(true); }

    // Polling + idle timer
    let state_poll = Arc::clone(&state);
    let proxy_poll = proxy.clone();
    std::thread::spawn(move || {
        let mut idle_since: Option<std::time::Instant> = None;
        let mut last_action: Option<std::time::Instant> = None;
        let mut last_running_switch = std::time::Instant::now();
        let bubbles = [
            "好无聊呀...","主人还在吗？","想和主人玩~","发呆中...",
            "（打哈欠）困了...","咦，有虫子飞过","（转圈圈）",
            "今天写了多少行代码呀？","（趴下）休息一会...","zZZ... 困...",
            "要不要喝杯咖啡？","（舔爪子）","窗外有鸟！",
            "（追尾巴）","第几个bug了？","喵~ 有人吗？",
            "想出去晒太阳...","（伸懒腰）","主人加油~",
        ];
        let actions = ["jumping", "waving", "review"];
        let mut tick: u64 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            tick += 1;
            let cur = state_poll.lock().unwrap_or_else(|e| e.into_inner()).current_state();
            let count = state_poll.lock().unwrap_or_else(|e| e.into_inner()).session_count();
            let _ = proxy_poll.send_event(UiCommand::SetState { state: cur, duration_ms: None });
            let _ = proxy_poll.send_event(UiCommand::SessionCount(count));

            if cur == LightState::Running && last_running_switch.elapsed().as_secs() >= 4 {
                last_running_switch = std::time::Instant::now();
                let _ = proxy_poll.send_event(UiCommand::SwitchRunning);
            }
            if cur != LightState::Idle {
                idle_since = None; last_action = None;
                continue;
            }
            // Idle: only run probability every 10 ticks (~10s)
            if tick % 10 != 0 { continue; }
            let now = std::time::Instant::now();
            let since = *idle_since.get_or_insert(now);
            let elapsed = now.duration_since(since).as_secs();
            if elapsed < 20 { continue; } // minimum 20s before first action
            if elapsed > 7200 { continue; } // stop after 2h

            // Cooldown: at least 20s between actions
            if let Some(t) = last_action {
                if t.elapsed().as_secs() < 20 { continue; }
            }

            // Probability decreases as idle time grows
            let prob = if elapsed < 300 { 40 }          // 0-5min: 40% per check
                  else if elapsed < 1800 { 15 }         // 5-30min: 15%
                  else { 5 };                            // 30min-2h: 5%

            let roll = {
                use std::hash::{BuildHasher, Hasher};
                let mut h = std::collections::hash_map::RandomState::new().build_hasher();
                h.write_u64(tick);
                h.finish() % 100
            };

            if roll >= prob { continue; }

            let idx = (elapsed as usize / 13) % bubbles.len();
            let s_idx = (elapsed as usize / 7) % actions.len();
            let _ = proxy_poll.send_event(UiCommand::IdleAction {
                action_state: actions[s_idx],
                bubble: bubbles[idx],
            });
            last_action = Some(now);
        }
    });

    // Event loop
    let mut last_state: Option<LightState> = None;
    let mut running_dir = "running-right";
    let mut exit_at: Option<std::time::Instant> = None;
    let state_exit = Arc::clone(&state);
    let webview = Some(webview);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(5));

        let (should_exit, count) = {
            let s = state_exit.lock().unwrap_or_else(|e| e.into_inner());
            (s.should_exit(), s.session_count())
        };
        if should_exit && count == 0 {
            if exit_at.is_none() { exit_at = Some(std::time::Instant::now()); }
            if exit_at.unwrap().elapsed() >= std::time::Duration::from_secs(2) {
                *control_flow = ControlFlow::Exit; return;
            }
        } else { exit_at = None; }

        match event {
            Event::UserEvent(cmd) => match cmd {
                UiCommand::SetState { state, duration_ms } => {
                    if last_state == Some(state) { return; }
                    last_state = Some(state);
                    if let Some(ref wv) = webview {
                        let sn = state_name(state);
                        let _ = wv.evaluate_script(&format!("setHookState('{}')", sn));
                        let _ = wv.evaluate_script(&format!("setState('{}',{})", sn, duration_ms.unwrap_or(0)));
                        if state != LightState::Idle {
                            let b = bubble_text(state);
                            if !b.is_empty() { let _ = wv.evaluate_script(&format!("setBubble('{}',0,true)", b)); }
                        } else {
                            let _ = wv.evaluate_script("setBubble('',0,false)");
                        }
                    }
                }
                UiCommand::SwitchRunning => {
                    if let Some(ref wv) = webview {
                        running_dir = if running_dir == "running-right" { "running-left" } else { "running-right" };
                        let _ = wv.evaluate_script(&format!("setState('{}',0)", running_dir));
                    }
                }
                UiCommand::SwitchPet { slug } => {
                    if let Some(bytes) = crate::webview::load_pet_bytes(&slug) {
                        save_pet_slug(&slug);
                        let mime = if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
                            "image/webp"
                        } else {
                            "image/png"
                        };
                        *sprite_data.lock().unwrap() = (bytes, mime);
                        if let Some(ref wv) = webview {
                            let slug_js = serde_json::to_string(&slug).unwrap_or_else(|_| "\"\"".into());
                            let bust = std::time::SystemTime::now()
                                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                .unwrap_or_default().as_millis();
                            let _ = wv.evaluate_script(&format!(
                                "CURRENT_SLUG={s};document.getElementById('pet').style.backgroundImage=\"url('/sprite?t={t}')\";",
                                s=slug_js, t=bust
                            ));
                        }
                    }
                }
                UiCommand::IdleAction { action_state, bubble } => {
                    if let Some(ref wv) = webview {
                        let _ = wv.evaluate_script(&format!("setState('{}',5000)", action_state));
                        let _ = wv.evaluate_script(&format!("setBubble('{}',4000)", bubble));
                    }
                }
                UiCommand::SessionCount(n) => {
                    if let Some(ref wv) = webview {
                        let _ = wv.evaluate_script(&format!("window.__sessions={}", n));
                    }
                }
                UiCommand::Quit => *control_flow = ControlFlow::Exit,
            },
            Event::WindowEvent { event: tao::event::WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });

    // event_loop.run never returns; Ok(()) is unreachable
    #[allow(unreachable_code)]
    Ok(())
}

pub fn start_detached_daemon(_port: u16) -> bool {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap_or_default());
    cmd.arg("--daemon").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)] { use std::os::windows::process::CommandExt; cmd.creation_flags(0x0000_0200 | 0x0000_0008); }
    cmd.spawn().is_ok()
}
pub fn fixed_port() -> u16 { FIXED_PORT }

fn save_pet_slug(slug: &str) {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("pet-slug"), slug);
}

fn load_pet_slug() -> Option<String> {
    std::fs::read_to_string(data_dir().join("pet-slug"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn data_dir() -> std::path::PathBuf {
    let home = crate::home_dir().unwrap_or_else(|| ".".into());
    std::path::PathBuf::from(home).join(".agent-critter").join("data")
}

fn save_scale(s: f32) {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("pet-scale"), s.to_string());
}

fn save_position(x: i32, y: i32) {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("position"), format!("{}\n{}", x, y));
}

fn load_position() -> Option<(i32, i32)> {
    let data = std::fs::read_to_string(data_dir().join("position")).ok()?;
    let mut lines = data.trim().lines();
    let x: i32 = lines.next()?.parse().ok()?;
    let y: i32 = lines.next()?.parse().ok()?;
    Some((x, y))
}