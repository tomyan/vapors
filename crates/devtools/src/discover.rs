//! Finding the page of a Chromium started with `--remote-debugging-port=0`:
//! the port it chose, from `DevToolsActivePort` in its profile, and the
//! page target's WebSocket, from `/json/list`. Both are polled by a
//! deadline: Chromium writes the file and opens the tab after it starts.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

const POLL: Duration = Duration::from_millis(50);

/// The port in `profile`'s `DevToolsActivePort`, once both its lines
/// (the port, then the browser's path) are written.
///
/// # Errors
/// No whole file by `deadline`.
pub fn port(profile: &Path, deadline: Duration) -> Result<u16, String> {
    let path = profile.join("DevToolsActivePort");
    let started = Instant::now();
    loop {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let mut lines = text.lines();
        if let (Some(port), Some(_browser)) = (lines.next(), lines.next())
            && let Ok(port) = port.trim().parse()
        {
            return Ok(port);
        }
        if started.elapsed() >= deadline {
            return Err(format!(
                "devtools: {} held no port within {deadline:?}",
                path.display()
            ));
        }
        thread::sleep(POLL);
    }
}

/// The first page target's WebSocket URL, polling `/json/list` on `port`.
///
/// # Errors
/// No page target by `deadline`, naming the last list read.
pub fn page_socket(port: u16, deadline: Duration) -> Result<String, String> {
    let started = Instant::now();
    loop {
        let body = json_list(port, deadline)?;
        if let Some(url) = first_page(&body) {
            return Ok(url);
        }
        if started.elapsed() >= deadline {
            return Err(format!(
                "devtools: no page target in /json/list within {deadline:?}; it read: {body}"
            ));
        }
        thread::sleep(POLL);
    }
}

fn first_page(body: &str) -> Option<String> {
    let targets: Value = serde_json::from_str(body).ok()?;
    let page = targets.as_array()?.iter().find(|t| t["type"] == "page")?;
    page["webSocketDebuggerUrl"].as_str().map(str::to_owned)
}

/// `GET /json/list`, read by its `Content-Length`: Chromium keeps the
/// connection open, so reading to its end would wait out the deadline.
fn json_list(port: u16, deadline: Duration) -> Result<String, String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .map_err(|e| format!("devtools: connecting to {port}: {e}"))?;
    stream
        .set_read_timeout(Some(deadline))
        .map_err(|e| e.to_string())?;
    // Chromium builds each target's WebSocket URL from this Host.
    let request = format!("GET /json/list HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let length = content_length(&mut reader)?;
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .map_err(|e| format!("devtools: reading /json/list: {e}"))?;
    String::from_utf8(body).map_err(|e| format!("devtools: /json/list is not UTF-8: {e}"))
}

fn content_length(reader: &mut impl BufRead) -> Result<usize, String> {
    let mut length = None;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| format!("devtools: reading /json/list: {e}"))?;
        let line = line.trim_end();
        if line.is_empty() {
            return length.ok_or_else(|| "devtools: /json/list had no Content-Length".to_owned());
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().ok();
        }
    }
}

#[cfg(test)]
#[path = "discover_tests.rs"]
mod tests;
