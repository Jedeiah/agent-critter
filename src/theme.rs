use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::assets::ThemeAssets;
use crate::state::LightState;

pub const FRAME_SIZE: i32 = 128;
pub const FRAME_COUNT: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeKind {
    CyberNeko,
    PixelSlime,
    QuantumCore,
    AbyssalEye,
    SteamGear,
    GhostFlame,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 6] = [
        Self::CyberNeko,
        Self::PixelSlime,
        Self::QuantumCore,
        Self::AbyssalEye,
        Self::SteamGear,
        Self::GhostFlame,
    ];

    pub fn dir_name(self) -> &'static str {
        match self {
            Self::CyberNeko => "cyber_neko",
            Self::PixelSlime => "pixel_slime",
            Self::QuantumCore => "quantum_core",
            Self::AbyssalEye => "abyssal_eye",
            Self::SteamGear => "steam_gear",
            Self::GhostFlame => "ghost_flame",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::CyberNeko => "赛博猫",
            Self::PixelSlime => "像素史莱姆",
            Self::QuantumCore => "量子核心",
            Self::AbyssalEye => "深渊之眼",
            Self::SteamGear => "蒸汽齿轮",
            Self::GhostFlame => "幽灵火焰",
        }
    }
}

fn state_filename(s: LightState) -> &'static str {
    match s {
        LightState::Idle => "idle",
        LightState::Running => "running",
        LightState::NeedConfirm => "need_confirm",
        LightState::ToolError => "tool_error",
        LightState::ErrorFinal => "error_final",
    }
}

pub struct SpriteStrip {
    rgba_data: Vec<u8>,
}

impl SpriteStrip {
    pub fn from_png(png_bytes: &[u8]) -> Option<Self> {
        let img = image::load_from_memory(png_bytes).ok()?;
        let rgba = img.to_rgba8();
        let expected_h = (FRAME_SIZE as u32) * (FRAME_COUNT as u32);
        if rgba.width() != FRAME_SIZE as u32 || rgba.height() != expected_h {
            eprintln!(
                "Bad sprite strip: {}x{}, expected {}x{}",
                rgba.width(),
                rgba.height(),
                FRAME_SIZE,
                expected_h
            );
            return None;
        }
        Some(Self {
            rgba_data: rgba.into_raw(),
        })
    }

    pub fn frame(&self, n: usize) -> &[u8] {
        let bytes_per_frame = (FRAME_SIZE * FRAME_SIZE * 4) as usize;
        let offset = n.min(FRAME_COUNT - 1) * bytes_per_frame;
        &self.rgba_data[offset..offset + bytes_per_frame]
    }
}

pub struct ThemeCache {
    strips: HashMap<LightState, SpriteStrip>,
}

impl ThemeCache {
    pub fn load(kind: ThemeKind) -> Option<Self> {
        let states = [
            LightState::Idle,
            LightState::Running,
            LightState::NeedConfirm,
            LightState::ToolError,
            LightState::ErrorFinal,
        ];
        let mut strips = HashMap::new();
        for &state in &states {
            let filename = format!("{}/{}.png", kind.dir_name(), state_filename(state));
            let file = ThemeAssets::get(&filename)?;
            let strip = SpriteStrip::from_png(&file.data)?;
            strips.insert(state, strip);
        }
        Some(Self { strips })
    }

    pub fn frame(&self, state: LightState, n: usize) -> Option<&[u8]> {
        self.strips.get(&state).map(|s| s.frame(n))
    }
}

pub struct AnimState {
    frame_index: usize,
    last_frame_time: Instant,
    pub blink_on: bool,
    last_blink_toggle: Instant,
    last_state: Option<LightState>,
}

impl AnimState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            frame_index: 0,
            last_frame_time: now,
            blink_on: true,
            last_blink_toggle: now,
            last_state: None,
        }
    }

    pub fn tick(&mut self, state: LightState) -> bool {
        let mut changed = false;

        if self.last_state != Some(state) {
            self.last_state = Some(state);
            self.frame_index = 1;
            self.blink_on = true;
            self.last_frame_time = Instant::now();
            self.last_blink_toggle = Instant::now();
            changed = true;
        }

        if self.last_frame_time.elapsed() >= Duration::from_millis(33) {
            // Frame 0 is dormant/dim — only used for blink-off. Normal playback: 1..29
            self.frame_index = if self.frame_index >= FRAME_COUNT - 1 { 1 } else { self.frame_index + 1 };
            self.last_frame_time = Instant::now();
            changed = true;
        }

        if state.should_blink() {
            let interval = Duration::from_millis(state.blink_interval_ms());
            if self.last_blink_toggle.elapsed() >= interval {
                self.blink_on = !self.blink_on;
                self.last_blink_toggle = Instant::now();
                changed = true;
            }
        } else {
            self.blink_on = true;
        }

        changed
    }

    pub fn display_frame(&self) -> usize {
        if self.blink_on {
            self.frame_index
        } else {
            0
        }
    }
}

pub fn save_theme_choice(theme: ThemeKind) {
    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(".agent-critter-theme")));
    if let Some(path) = path {
        std::fs::write(path, theme.dir_name()).ok();
    }
}

pub fn load_theme_choice() -> ThemeKind {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(".agent-critter-theme")))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| {
            ThemeKind::ALL
                .iter()
                .find(|t| t.dir_name() == s.trim())
                .copied()
        })
        .unwrap_or(ThemeKind::QuantumCore)
}
