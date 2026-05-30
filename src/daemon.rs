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
    let home = match std::env::var("HOME") { Ok(h) => h, Err(_) => return pets };
    for base in &[format!("{}/.codex/pets", home), format!("{}/.petdex/pets", home)] {
        let dir = match std::fs::read_dir(base) { Ok(d) => d, Err(_) => continue };
        for entry in dir.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let slug = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
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
    Move { dx: i32, dy: i32 },
    SwitchPet { slug: String },
    IdleAction { action_state: &'static str, bubble: &'static str },
    Focus,
    Quit,
}

fn state_name(s: LightState) -> &'static str {
    match s {
        LightState::Idle => "idle", LightState::Running => "running",
        LightState::NeedConfirm => "waving", LightState::ToolError => "review",
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
    let window = WindowBuilder::new()
        .with_inner_size(tao::dpi::LogicalSize::new(140.0, 180.0))
        .with_decorations(false).with_transparent(true).with_always_on_top(true)
        .with_resizable(false)
        .build(&event_loop).expect("window");

    // Restore saved position
    if let Some((x, y)) = load_position() {
        window.set_outer_position(tao::dpi::PhysicalPosition::new(x, y));
    }

    #[cfg(target_os = "macos")]
    { use tao::platform::macos::EventLoopExtMacOS;
      event_loop.set_activation_policy(tao::platform::macos::ActivationPolicy::Accessory); }

    // Load pet
    let (sheet, slug) = crate::webview::find_first_pet()
        .unwrap_or_else(|| (include_bytes!("../assets/default_spritesheet.webp").to_vec(), "default".into()));
    let pets = list_pets();
    let pets_json = serde_json::to_string(&pets).unwrap_or_default();
    let html = crate::webview::build_page(&sheet, &slug, &pets_json);

    // WebView
    let proxy_ipc = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_transparent(true).with_html(&html)
        .with_ipc_handler(move |msg| {
            if msg.body() == "quit" { std::process::exit(0); }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(msg.body()) {
                if let Some(slug) = v.get("theme").and_then(|t| t.as_str()) {
                    let _ = proxy_ipc.send_event(UiCommand::SwitchPet { slug: slug.into() });
                }
                if v.get("type").and_then(|t| t.as_str()) == Some("focus") {
                    let _ = proxy_ipc.send_event(UiCommand::Focus);
                }
                if v.get("type").and_then(|t| t.as_str()) == Some("move") {
                    let dx = v.get("dx").and_then(|d| d.as_i64()).unwrap_or(0) as i32;
                    let dy = v.get("dy").and_then(|d| d.as_i64()).unwrap_or(0) as i32;
                    let _ = proxy_ipc.send_event(UiCommand::Move { dx, dy });
                }
            }
        })
        .build(&window).expect("webview");
    window.set_visible(true);

    // macOS: WKWebView drawsBackground = NO (must be AFTER webview build)
    #[cfg(target_os = "macos")]
    { use tao::platform::macos::WindowExtMacOS;
      let ns: *mut std::ffi::c_void = window.ns_window();
      if !ns.is_null() { unsafe {
        let win: &objc::runtime::Object = &*(ns as *const objc::runtime::Object);
        let _: () = objc::msg_send![win, setOpaque: false];
        let c: *mut objc::runtime::Object = objc::msg_send![objc::class!(NSColor), clearColor];
        let _: () = objc::msg_send![win, setBackgroundColor: c];
        let cv: *mut objc::runtime::Object = objc::msg_send![win, contentView];
        if !cv.is_null() {
          let subs: *mut objc::runtime::Object = objc::msg_send![cv, subviews];
          let n: usize = objc::msg_send![subs, count];
          for i in 0..n {
            let v: *mut objc::runtime::Object = objc::msg_send![subs, objectAtIndex: i];
            let no: *mut objc::runtime::Object = objc::msg_send![objc::class!(NSNumber), numberWithBool: false];
            let k: *mut objc::runtime::Object = objc::msg_send![objc::class!(NSString), stringWithUTF8String: b"drawsBackground\0".as_ptr() as *const i8];
            let _: () = objc::msg_send![v, setValue: no forKey: k];
          }
        }
      }}
    }

    // Polling + idle timer
    let state_poll = Arc::clone(&state);
    let proxy_poll = proxy.clone();
    std::thread::spawn(move || {
        let mut idle_since: Option<std::time::Instant> = None;
        let mut last_action: Option<std::time::Instant> = None;
        let mut action_count: u32 = 0;
        let idle_bubbles = [
            "好无聊呀...",
            "主人还在吗？",
            "想和主人玩~",
            "发呆中...",
            "喵~ 有人吗？",
            "（打哈欠）困了...",
            "咦，有虫子飞过",
            "（转圈圈）",
            "今天写了多少行代码呀？",
            "（趴下）休息一会...",
            "要不要喝杯咖啡？",
            "zZZ... 没有，我没睡着！",
        ];
        let idle_states = ["jumping", "waving"];
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let cur = state_poll.lock().unwrap().current_state();
            let _ = proxy_poll.send_event(UiCommand::SetState { state: cur, duration_ms: None });

            // Idle action logic
            if cur == LightState::Idle {
                if idle_since.is_none() {
                    idle_since = Some(std::time::Instant::now());
                    action_count = 0;
                }
                let idle_dur = idle_since.unwrap().elapsed().as_secs();
                if idle_dur >= 30 && action_count < 2 {
                    let should_act = match last_action {
                        None => true,
                        Some(t) => t.elapsed().as_secs() >= 30 + (idle_dur as u64 % 30),
                    };
                    if should_act {
                        let idx = (idle_dur as usize / 7) % idle_bubbles.len();
                        let state_idx = (idle_dur as usize / 13) % idle_states.len();
                        let _ = proxy_poll.send_event(UiCommand::IdleAction {
                            action_state: idle_states[state_idx],
                            bubble: idle_bubbles[idx],
                        });
                        last_action = Some(std::time::Instant::now());
                        action_count += 1;
                    }
                }
            } else {
                idle_since = None;
                last_action = None;
                action_count = 0;
            }
        }
    });

    // Event loop
    let mut last_state = None;
    let mut exit_at: Option<std::time::Instant> = None;
    let state_exit = Arc::clone(&state);
    let webview = Some(webview);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(20));

        let (should_exit, count) = {
            let s = state_exit.lock().unwrap();
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
                        let _ = wv.evaluate_script(&format!("setState('{}',{})", state_name(state), duration_ms.unwrap_or(0)));
                        // Bubble for non-idle state changes
                        if state != LightState::Idle {
                            let b = bubble_text(state);
                            if !b.is_empty() { let _ = wv.evaluate_script(&format!("setBubble('{}',3000)", b)); }
                        }
                    }
                }
                UiCommand::Move { dx, dy } => {
                    let pos = window.outer_position().unwrap_or_default();
                    let new_x = pos.x + dx;
                    let new_y = pos.y + dy;
                    window.set_outer_position(tao::dpi::PhysicalPosition::new(new_x, new_y));
                    save_position(new_x, new_y);
                }
                UiCommand::SwitchPet { slug } => {
                    if let Some(bytes) = crate::webview::load_pet_bytes(&slug) {
                        let pj = serde_json::to_string(&list_pets()).unwrap_or_default();
                        if let Some(ref wv) = webview {
                            let _ = wv.load_html(&crate::webview::build_page(&bytes, &slug, &pj));
                        }
                    }
                }
                UiCommand::Focus => {
                    #[cfg(target_os = "macos")]
                    { use tao::platform::macos::WindowExtMacOS;
                      let _ = window.ns_window(); // keeps reference alive
                      unsafe {
                        let ns = window.ns_window() as *mut objc::runtime::Object;
                        if !ns.is_null() {
                          let _: () = objc::msg_send![ns, makeKeyAndOrderFront: std::ptr::null::<objc::runtime::Object>()];
                          let app: *mut objc::runtime::Object = objc::msg_send![objc::class!(NSApplication), sharedApplication];
                          let _: () = objc::msg_send![app, activateIgnoringOtherApps: true];
                        }
                      }
                    }
                }
                UiCommand::IdleAction { action_state, bubble } => {
                    if let Some(ref wv) = webview {
                        let _ = wv.evaluate_script(&format!("setState('{}',2000)", action_state));
                        let _ = wv.evaluate_script(&format!("setBubble('{}',3000)", bubble));
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

fn pos_path() -> Option<std::path::PathBuf> {
    let dir = std::env::current_dir().ok()?.join("data");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("position"))
}

fn save_position(x: i32, y: i32) {
    if let Some(p) = pos_path() {
        let _ = std::fs::write(p, format!("{}\n{}", x, y));
    }
}

fn load_position() -> Option<(i32, i32)> {
    let data = std::fs::read_to_string(pos_path()?).ok()?;
    let mut lines = data.trim().lines();
    let x: i32 = lines.next()?.parse().ok()?;
    let y: i32 = lines.next()?.parse().ok()?;
    Some((x, y))
}
