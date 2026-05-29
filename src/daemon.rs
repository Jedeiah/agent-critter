use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use crate::server::start_server;
use crate::state::StateMachine;
use crate::ui::run_ui;

const FIXED_PORT: u16 = 7890;

pub fn run_daemon(port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .map_err(|_| format!("Port {} is already in use, daemon may already be running", port))?;

    let state = Arc::new(Mutex::new(StateMachine::new()));
    let state_clone = Arc::clone(&state);

    std::thread::spawn(move || {
        start_server(listener, state_clone);
    });

    run_ui(state);

    // UI 关闭后强制退出整个进程，结束 server 线程
    std::process::exit(0);
}

pub fn start_detached_daemon(_port: u16) -> bool {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return false,
    };

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("--daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0200 | 0x0000_0008);
    }

    cmd.spawn().is_ok()
}

pub fn fixed_port() -> u16 {
    FIXED_PORT
}
