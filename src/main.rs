use std::env;
use std::io::{self, Read};
use std::time::Duration;
use agent_critter::client::{send_event_str, send_raw_json};

use agent_critter::daemon::{run_daemon, start_detached_daemon, fixed_port, is_manual_quit, clear_manual_quit};

fn read_stdin_and_send(port: u16) {
    let mut stdin_bytes = Vec::new();
    if io::stdin().read_to_end(&mut stdin_bytes).is_err() {
        return;
    }
    if stdin_bytes.is_empty() || stdin_bytes.iter().all(|&b| b.is_ascii_whitespace()) {
        return;
    }

    // 用户手动退出后，只有新会话开始才允许重新启动
    if is_manual_quit() {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&stdin_bytes) {
            if v.get("hook_event_name").and_then(|n| n.as_str()) == Some("SessionStart") {
                clear_manual_quit(); // 新会话：允许重启
            } else {
                return; // 其他 hook 不重启
            }
        } else {
            return;
        }
    }

    // Try sending; if daemon not running, start it and poll up to ~2s
    let mut daemon_started = false;
    for _ in 0..20 {
        match send_raw_json(port, &stdin_bytes) {
            Ok(()) => return,
            Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                if !daemon_started {
                    start_detached_daemon(port);
                    daemon_started = true;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let port = fixed_port();

    if args.len() >= 2 && args[1] == "--daemon" {
        match run_daemon(port) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if args.len() >= 2 && args[1] == "--hook" {
        read_stdin_and_send(port);
        return;
    }

    // Legacy: --event <name>
    let session_id = env::var("CLAUDE_SESSION_ID").unwrap_or_else(|_| "default_session".to_string());
    let event = if args.len() >= 3 && args[1] == "--event" {
        &args[2]
    } else {
        "idle"
    };

    let mut daemon_started = false;
    for _ in 0..20 {
        match send_event_str(port, event, &session_id) {
            Ok(()) => return,
            Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                if !daemon_started {
                    start_detached_daemon(port);
                    daemon_started = true;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return,
        }
    }
}
