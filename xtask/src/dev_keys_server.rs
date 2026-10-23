//! The loopback server behind `/dev/keys.json`: trunk proxies the
//! debug page's request here.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use crate::dev_keys::{DevKey, read};

/// The one path answered.
pub const KEYS_PATH: &str = "/dev/keys.json";

/// How long one request may take to arrive. trunk's proxy on the same
/// machine sends it at once; this only stops a stuck connection from
/// holding the server's one thread. Not measured.
const REQUEST_DEADLINE: Duration = Duration::from_secs(5);

/// Serve `keys` on a free loopback port; returns the port.
///
/// # Errors
///
/// No loopback port could be bound.
pub fn start(keys: Vec<DevKey>) -> Result<u16, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| format!("no loopback port for the dev keys: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("the dev keys' port: {e}"))?
        .port();
    std::thread::spawn(move || println!("{}", serve(listener.incoming(), &keys)));
    Ok(port)
}

/// Answer each connection until one cannot be accepted; returns the
/// FAIL line saying the keys are no longer served. The page then gets
/// trunk's proxy error, which its fetch reports on the error row.
fn serve(connections: impl Iterator<Item = std::io::Result<TcpStream>>, keys: &[DevKey]) -> String {
    for connection in connections {
        match connection {
            Ok(stream) => answer_one(stream, keys),
            Err(error) => {
                return format!(
                    "FAIL dev keys: accepting a connection failed ({error}); {KEYS_PATH} is no longer served; fix: restart cargo xtask dev"
                );
            }
        }
    }
    format!(
        "FAIL dev keys: the listener closed; {KEYS_PATH} is no longer served; fix: restart cargo xtask dev"
    )
}

/// Answer one connection; a failed write means the page went away.
fn answer_one(mut stream: TcpStream, keys: &[DevKey]) {
    let _ = stream.set_read_timeout(Some(REQUEST_DEADLINE));
    let (status, body) = match request_path(&stream) {
        Some(path) if path == KEYS_PATH => answer(keys),
        _ => ("404 Not Found", "not found".to_owned()),
    };
    let kind = if status.starts_with("200") {
        "application/json"
    } else {
        "text/plain; charset=utf-8"
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

/// The path of a `GET`, without its query.
fn request_path(stream: &TcpStream) -> Option<String> {
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    let mut words = line.split_whitespace();
    if words.next()? != "GET" {
        return None;
    }
    let target = words.next()?;
    Some(target.split('?').next().unwrap_or(target).to_owned())
}

/// The status and body for `/dev/keys.json`: every key read now, a 500
/// naming the first file that failed, or a 404 without keys.
fn answer(keys: &[DevKey]) -> (&'static str, String) {
    if keys.is_empty() {
        return (
            "404 Not Found",
            "no --key given to cargo xtask dev".to_owned(),
        );
    }
    let mut map = serde_json::Map::new();
    for key in keys {
        match read(key) {
            Ok(text) => map.insert(key.provider.clone(), text.into()),
            Err(why) => return ("500 Internal Server Error", why),
        };
    }
    ("200 OK", serde_json::Value::Object(map).to_string())
}

#[cfg(test)]
#[path = "dev_keys_server_tests.rs"]
mod tests;
