#![allow(unexpected_cfgs)]

#[cfg(target_os = "macos")]
#[macro_use] extern crate objc;

pub mod assets;
pub mod client;
pub mod daemon;
pub mod hook;
pub mod server;
pub mod state;
pub mod webview;

pub fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
}
