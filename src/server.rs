use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use crate::hook::{map_hook_event, HookPayload};
use crate::state::StateMachine;

/// 修复：接收已绑定的 listener，不再内部二次 bind
pub fn start_server(listener: TcpListener, state: Arc<Mutex<StateMachine>>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state_clone = Arc::clone(&state);
                std::thread::spawn(move || {
                    handle_client(stream, state_clone);
                });
            }
            Err(e) => {
                eprintln!("Accept failed: {}", e);
            }
        }
    }
}

fn handle_client(stream: TcpStream, state: Arc<Mutex<StateMachine>>) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let payload: HookPayload = match serde_json::from_str(trimmed) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("JSON parse error: {}", e);
                continue;
            }
        };

        let session_id = payload
            .session_id
            .clone()
            .unwrap_or_else(|| "default_session".to_string());
        let event = map_hook_event(&payload);

        let mut sm = state.lock().unwrap_or_else(|e| e.into_inner());
        sm.handle_event(&session_id, event);
    }
}
