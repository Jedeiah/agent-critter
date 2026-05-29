use std::env;
use std::io::{self, Read};
use std::time::Duration;
use agent_critter::client::{send_event_str, send_raw_json};

use agent_critter::daemon::{run_daemon, start_detached_daemon, fixed_port};

fn read_stdin_and_send(port: u16) {
    let mut stdin_bytes = Vec::new();
    if io::stdin().read_to_end(&mut stdin_bytes).is_err() {
        return;
    }
    if stdin_bytes.is_empty() || stdin_bytes.iter().all(|&b| b.is_ascii_whitespace()) {
        return;
    }

    // Try sending; if daemon not running, start it and poll up to ~2s
    for _ in 0..20 {
        match send_raw_json(port, &stdin_bytes) {
            Ok(()) => return,
            Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                start_detached_daemon(port);
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

    for _ in 0..20 {
        match send_event_str(port, event, &session_id) {
            Ok(()) => return,
            Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                start_detached_daemon(port);
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return,
        }
    }
}
