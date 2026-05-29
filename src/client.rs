use std::io::{self, Write};
use std::net::TcpStream;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct EventPayload {
    pub event: String,
    pub session_id: String,
}

pub fn send_event_str(port: u16, event: &str, session_id: &str) -> io::Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect(&addr)?;

    let payload = EventPayload {
        event: event.to_string(),
        session_id: session_id.to_string(),
    };

    let json_line = serde_json::to_string(&payload)?;
    writeln!(stream, "{}", json_line)?;
    stream.flush()?;

    Ok(())
}

/// Send raw JSON bytes directly (stdin passthrough mode).
pub fn send_raw_json(port: u16, data: &[u8]) -> io::Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect(&addr)?;
    stream.write_all(data)?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}
