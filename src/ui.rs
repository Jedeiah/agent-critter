use fltk::{
    app,
    draw,
    enums::*,
    frame::Frame,
    prelude::*,
    window::Window,
};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::state::{LightState, StateMachine};
use crate::theme::{AnimState, ThemeCache, ThemeKind, FRAME_SIZE};

const PANEL_W: i32 = 150;
const PANEL_H: i32 = 68;
const PAD: i32 = 12;
const RADIUS: i32 = 8;

fn c(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r, g, b)
}

struct StateInfo {
    color: (u8, u8, u8),
    label: &'static str,
    sub: &'static str,
}

fn state_info(s: LightState) -> StateInfo {
    match s {
        LightState::Idle => StateInfo {
            color: (0, 230, 118),
            label: "空闲",
            sub: "等待新任务",
        },
        LightState::Running => StateInfo {
            color: (0, 230, 118),
            label: "运行中",
            sub: "Claude 处理中",
        },
        LightState::NeedConfirm => StateInfo {
            color: (255, 234, 0),
            label: "等待确认",
            sub: "需要您的操作",
        },
        LightState::ToolError => StateInfo {
            color: (255, 100, 50),
            label: "工具错误",
            sub: "可重试",
        },
        LightState::ErrorFinal => StateInfo {
            color: (255, 23, 68),
            label: "严重错误",
            sub: "需人工处理",
        },
    }
}

static PANEL_OPEN: AtomicBool = AtomicBool::new(false);

struct Drag {
    ox: i32,
    oy: i32,
    active: bool,
    total_dx: i32,
    total_dy: i32,
}

pub fn run_ui(state_machine: Arc<Mutex<StateMachine>>) {
    let _app = app::App::default();
    app::set_visual(Mode::Rgb8).ok();

    let initial_theme = crate::theme::load_theme_choice();
    let theme_cache: Arc<Mutex<Option<ThemeCache>>> =
        Arc::new(Mutex::new(ThemeCache::load(initial_theme)));
    let current_theme = Arc::new(Mutex::new(initial_theme));
    let anim = Arc::new(Mutex::new(AnimState::new()));

    let win_size = FRAME_SIZE;

    // Main window
    let mut win = Window::new(0, 0, win_size, win_size, "");
    win.set_border(false);
    win.make_modal(false);
    let mut frame = Frame::default().with_size(win_size, win_size);
    win.end();
    win.show();

    let sw = app::screen_size().0 as i32;
    win.set_pos(sw - win_size - 18, 20);
    set_topmost_transparent(&win);

    // Draw callback
    let tc_draw = theme_cache.clone();
    let sm_draw = state_machine.clone();
    let anim_draw = anim.clone();

    frame.draw(move |_f| {
        let state = {
            let s = sm_draw.lock().unwrap_or_else(|e| e.into_inner());
            s.current_state()
        };
        let frame_idx = {
            let a = anim_draw.lock().unwrap();
            a.display_frame()
        };
        draw::set_draw_color(c(1, 0, 1));
        draw::draw_rectf(0, 0, FRAME_SIZE, FRAME_SIZE);
        let tc = tc_draw.lock().unwrap();
        match &*tc {
            Some(cache) => {
                if let Some(rgba) = cache.frame(state, frame_idx) {
                    draw::draw_image(rgba, 0, 0, FRAME_SIZE, FRAME_SIZE, ColorDepth::Rgba8).ok();
                } else {
                    draw_fallback(state);
                }
            }
            None => draw_fallback(state),
        }
    });

    // Panel
    let panel_x = sw - win_size - 18 - PANEL_W - 8;
    let mut panel = Window::new(panel_x, 20, PANEL_W, PANEL_H, "");
    panel.set_border(false);
    panel.set_color(Color::TransparentBg);

    let exit_hover = Arc::new(AtomicBool::new(false));
    let exit_hover_draw = exit_hover.clone();
    let state_for_draw = state_machine.clone();

    panel.draw(move |p| {
        let cur = {
            let s = state_for_draw.lock().unwrap_or_else(|e| e.into_inner());
            s.current_state()
        };
        let hover = exit_hover_draw.load(Ordering::Relaxed);
        draw_panel(p.w(), p.h(), cur, hover);
    });

    let exit_hover_handle = exit_hover.clone();
    panel.handle(move |p, ev| {
        let btn_x = PAD;
        let btn_w = PANEL_W - PAD * 2;
        let btn_y = PANEL_H - 28;
        let btn_h = 20;

        match ev {
            Event::Enter | Event::Move => {
                let mx = app::event_x();
                let my = app::event_y();
                let in_btn =
                    mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
                let prev = exit_hover_handle.swap(in_btn, Ordering::Relaxed);
                if prev != in_btn {
                    p.redraw();
                }
                true
            }
            Event::Leave => {
                if exit_hover_handle.swap(false, Ordering::Relaxed) {
                    p.redraw();
                }
                true
            }
            Event::Push => {
                let mx = app::event_x();
                let my = app::event_y();
                if mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h {
                    app::quit();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    });

    panel.end();

    // Drag state
    let drag = Arc::new(Mutex::new(Drag {
        ox: 0,
        oy: 0,
        active: false,
        total_dx: 0,
        total_dy: 0,
    }));
    let drag_handle = drag.clone();
    let drag_handle2 = drag.clone();
    let drag_handle3 = drag.clone();
    let mut panel_handle = panel.clone();
    let mut panel_handle2 = panel.clone();

    // Theme selector state
    let theme_for_menu = current_theme.clone();
    let cache_for_menu = theme_cache.clone();

    // Store click position for theme popup
    let popup_x = Arc::new(AtomicI32::new(0));
    let popup_y = Arc::new(AtomicI32::new(0));
    let popup_x_h = popup_x.clone();
    let popup_y_h = popup_y.clone();

    win.handle(move |w, ev| {
        match ev {
            Event::Push if app::event_button() == 1 => {
                let mut d = drag_handle.lock().unwrap();
                d.ox = app::event_x_root();
                d.oy = app::event_y_root();
                d.active = true;
                d.total_dx = 0;
                d.total_dy = 0;
                true
            }
            Event::Drag => {
                let mut d = drag_handle2.lock().unwrap();
                if d.active {
                    let mx = app::event_x_root();
                    let my = app::event_y_root();
                    let dx = mx - d.ox;
                    let dy = my - d.oy;
                    d.total_dx += dx.abs();
                    d.total_dy += dy.abs();
                    d.ox = mx;
                    d.oy = my;
                    w.set_pos(w.x() + dx, w.y() + dy);
                    if PANEL_OPEN.load(Ordering::Relaxed) {
                        panel_handle.set_pos(w.x() - PANEL_W - 8, w.y());
                    }
                }
                true
            }
            Event::Released if app::event_button() == 1 => {
                let d = drag_handle3.lock().unwrap();
                let was_click = d.total_dx < 4 && d.total_dy < 4;
                drop(d);
                drag_handle3.lock().unwrap().active = false;

                if was_click {
                    let open = PANEL_OPEN.load(Ordering::Relaxed);
                    if open {
                        PANEL_OPEN.store(false, Ordering::Relaxed);
                        panel_handle2.hide();
                    } else {
                        PANEL_OPEN.store(true, Ordering::Relaxed);
                        panel_handle2.set_pos(w.x() - PANEL_W - 8, w.y());
                        panel_handle2.show();
                        set_topmost(&panel_handle2);
                        panel_handle2.redraw();
                    }
                }
                true
            }
            Event::Push if app::event_button() == 3 => {
                popup_x_h.store(app::event_x_root(), Ordering::Relaxed);
                popup_y_h.store(app::event_y_root(), Ordering::Relaxed);
                true
            }
            Event::Released if app::event_button() == 3 => {
                show_theme_popup(
                    popup_x.load(Ordering::Relaxed),
                    popup_y.load(Ordering::Relaxed),
                    &theme_for_menu,
                    &cache_for_menu,
                );
                true
            }
            _ => false,
        }
    });

    // Idle loop
    let sm = state_machine.clone();
    let anim_idle = anim.clone();
    let mut exit_started = false;
    let mut exit_timer: Option<std::time::Instant> = None;
    let mut panel_idle = panel.clone();
    let mut frame_idle = frame.clone();

    app::add_idle3(move |_| {
        let (state, should_exit, session_count) = {
            let s = sm.lock().unwrap_or_else(|e| e.into_inner());
            (s.current_state(), s.should_exit(), s.session_count())
        };

        if should_exit && session_count == 0 && !exit_started {
            exit_started = true;
            exit_timer = Some(std::time::Instant::now());
        }
        if exit_started && session_count > 0 {
            exit_started = false;
            exit_timer = None;
        }
        if let Some(t) = exit_timer {
            if t.elapsed() >= Duration::from_secs(2) {
                panel_idle.hide();
                app::quit();
                return;
            }
        }

        let mut a = anim_idle.lock().unwrap();
        let needs_redraw = a.tick(state);
        drop(a);

        if needs_redraw {
            frame_idle.redraw();
            if PANEL_OPEN.load(Ordering::Relaxed) {
                panel_idle.redraw();
            }
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    });

    app::run().expect("FLTK 运行失败");
}

fn show_theme_popup(
    x: i32,
    y: i32,
    current: &Arc<Mutex<ThemeKind>>,
    cache: &Arc<Mutex<Option<ThemeCache>>>,
) {
    let item_h: i32 = 32;
    let count = ThemeKind::ALL.len() as i32;
    let menu_w: i32 = 160;
    let menu_h: i32 = count * item_h + 12;

    let mut popup = Window::new(x, y, menu_w, menu_h, "");
    popup.set_border(false);
    popup.set_color(Color::TransparentBg);

    let hover_idx = Arc::new(AtomicI32::new(-1));
    let cur_kind = *current.lock().unwrap();

    let hover_draw = hover_idx.clone();
    popup.draw(move |_p| {
        let hi = hover_draw.load(Ordering::Relaxed);
        draw_theme_menu(menu_w, menu_h, item_h, cur_kind, hi);
    });

    let hover_handle = hover_idx.clone();
    let current_c = current.clone();
    let cache_c = cache.clone();

    popup.handle(move |p, ev| {
        match ev {
            Event::Enter | Event::Move => {
                let my = app::event_y() - 6;
                let idx = my / item_h;
                let new_idx = if idx >= 0 && idx < count { idx } else { -1 };
                let prev = hover_handle.swap(new_idx, Ordering::Relaxed);
                if prev != new_idx {
                    p.redraw();
                }
                true
            }
            Event::Leave => {
                hover_handle.store(-1, Ordering::Relaxed);
                p.redraw();
                true
            }
            Event::Push => {
                let my = app::event_y() - 6;
                let idx = my / item_h;
                if idx >= 0 && (idx as usize) < ThemeKind::ALL.len() {
                    let tk = ThemeKind::ALL[idx as usize];
                    *current_c.lock().unwrap() = tk;
                    *cache_c.lock().unwrap() = ThemeCache::load(tk);
                    crate::theme::save_theme_choice(tk);
                }
                p.hide();
                true
            }
            Event::Unfocus => {
                p.hide();
                true
            }
            _ => false,
        }
    });

    popup.end();
    popup.show();
    set_topmost(&popup);
}

fn draw_theme_menu(w: i32, h: i32, item_h: i32, current: ThemeKind, hover_idx: i32) {
    let bg = c(18, 22, 40);
    let border = c(55, 65, 100);
    draw::set_draw_color(bg);
    draw::draw_rounded_rectf(0, 0, w, h, 8);
    draw::set_draw_color(border);
    draw::draw_rounded_rect(0, 0, w, h, 8);

    for (i, &tk) in ThemeKind::ALL.iter().enumerate() {
        let y = 6 + i as i32 * item_h;
        let is_current = tk == current;
        let is_hover = i as i32 == hover_idx;

        if is_hover {
            draw::set_draw_color(c(35, 42, 72));
            draw::draw_rectf(4, y, w - 8, item_h);
        }

        if is_current {
            draw::set_draw_color(c(0, 200, 120));
            draw::draw_pie(12, y + item_h / 2 - 3, 6, 6, 0.0, 360.0);
        }

        let text_c = if is_current {
            c(220, 240, 255)
        } else if is_hover {
            c(200, 210, 230)
        } else {
            c(140, 150, 175)
        };
        draw::set_draw_color(text_c);
        draw::set_font(Font::Helvetica, 12);
        draw::draw_text2(tk.display_name(), 24, y, w - 32, item_h, Align::Left);
    }
}

fn draw_fallback(state: LightState) {
    let info = state_info(state);
    draw::set_draw_color(c(8, 10, 18));
    draw::draw_rectf(0, 0, FRAME_SIZE, FRAME_SIZE);
    let (cr, cg, cb) = info.color;
    draw::set_draw_color(c(cr, cg, cb));
    draw::draw_pie(24, 24, 80, 80, 0.0, 360.0);
    draw::set_draw_color(c(255, 255, 255));
    draw::set_font(Font::HelveticaBold, 11);
    draw::draw_text2(info.label, 0, 100, FRAME_SIZE, 20, Align::Center);
}

fn draw_panel(w: i32, h: i32, state: LightState, exit_hover: bool) {
    let info = state_info(state);
    let (sr, sg, sb) = info.color;

    draw::set_draw_color(c(16, 19, 34));
    draw::draw_rounded_rectf(0, 0, w, h, RADIUS);
    draw::set_draw_color(c(32, 38, 58));
    draw::draw_rounded_rect(0, 0, w, h, RADIUS);

    draw::set_draw_color(c(sr, sg, sb));
    draw::draw_pie(PAD, 14, 8, 8, 0.0, 360.0);

    draw::set_draw_color(c(220, 228, 250));
    draw::set_font(Font::HelveticaBold, 13);
    draw::draw_text2(info.label, PAD + 14, 10, w - PAD * 2 - 14, 16, Align::Left);

    draw::set_draw_color(c(90, 98, 125));
    draw::set_font(Font::Helvetica, 10);
    draw::draw_text2(info.sub, PAD + 14, 28, w - PAD * 2 - 14, 14, Align::Left);

    let exit_y = h - 22;
    if exit_hover {
        draw::set_draw_color(c(220, 80, 90));
    } else {
        draw::set_draw_color(c(70, 76, 100));
    }
    draw::set_font(Font::Helvetica, 9);
    draw::draw_text2("\u{00d7} 退出", PAD, exit_y, w - PAD * 2, 16, Align::Right);
}

fn set_topmost(win: &Window) {
    set_topmost_inner(win, false);
}

fn set_topmost_transparent(win: &Window) {
    set_topmost_inner(win, true);
}

fn set_topmost_inner(win: &Window, transparent: bool) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let hwnd = win.raw_handle() as HWND;
        if !hwnd.is_null() {
            unsafe {
                let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
                let mut flags = ex | WS_EX_TOOLWINDOW as i32;
                if transparent {
                    flags |= WS_EX_LAYERED as i32;
                }
                SetWindowLongW(hwnd, GWL_EXSTYLE, flags);
                if transparent {
                    SetLayeredWindowAttributes(hwnd, 0x00010001, 0, LWA_COLORKEY);
                }
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let nswin = win.raw_handle() as *mut objc::runtime::Object;
        if !nswin.is_null() {
            unsafe {
                use objc::*;
                let _: () = msg_send![nswin, setLevel: 5i64];
                let b: u64 = (1 << 0) | (1 << 4);
                let _: () = msg_send![nswin, setCollectionBehavior: b];
                if transparent {
                    let _: () = msg_send![nswin, setOpaque: false];
                    let ns_clear: *mut objc::runtime::Object =
                        msg_send![class!(NSColor), clearColor];
                    let _: () = msg_send![nswin, setBackgroundColor: ns_clear];
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let _ = transparent;
        std::process::Command::new("wmctrl")
            .args(["-r", ":ACTIVE:", "-b", "add,above"])
            .spawn()
            .ok();
    }
}

======================================================================
  导出完成
  共 19 个文件，3989 行代码
======================================================================
