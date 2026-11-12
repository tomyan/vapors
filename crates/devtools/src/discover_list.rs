//! Reading `/json/list` from Chromium's `DevTools` port: one GET, its
//! response head and its body by `Content-Length`.

use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Instant;

use crate::connect::left;

/// `GET /json/list`; `None` while nothing listens on `port`, and when
/// the deadline passes before the answer is read: that is the deadline's
/// refusal, which quotes the last list, not a failure of its own. The
/// body is read by its `Content-Length`: Chromium keeps the connection
/// open.
pub(crate) fn json_list(port: u16, until: Instant) -> Result<Option<String>, String> {
    match request_list(port, until) {
        Ok(body) => Ok(Some(body)),
        Err(Failure::NotThere | Failure::TimedOut) => Ok(None),
        Err(Failure::Refused(why)) => Err(why),
    }
}

/// Why `/json/list` was not read.
enum Failure {
    /// Nothing listens on the port yet.
    NotThere,
    /// The deadline passed before the answer was read.
    TimedOut,
    Refused(String),
}

impl From<String> for Failure {
    fn from(why: String) -> Self {
        Self::Refused(why)
    }
}

fn request_list(port: u16, until: Instant) -> Result<String, Failure> {
    let budget = left(until).map_err(|_| Failure::TimedOut)?;
    let mut stream = match TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), budget) {
        Ok(stream) => stream,
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => return Err(Failure::NotThere),
        Err(e) if timed_out(&e) => return Err(Failure::TimedOut),
        Err(e) => return Err(format!("devtools: connecting to {port}: {e}").into()),
    };
    let budget = left(until).map_err(|_| Failure::TimedOut)?;
    stream
        .set_read_timeout(Some(budget))
        .map_err(|e| e.to_string())?;
    // Chromium builds each target's WebSocket URL from this Host.
    let request = format!("GET /json/list HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let length = head(&mut reader)?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(|e| read_failure(&e))?;
    String::from_utf8(body).map_err(|e| format!("devtools: /json/list is not UTF-8: {e}").into())
}

fn timed_out(e: &std::io::Error) -> bool {
    matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

fn read_failure(e: &std::io::Error) -> Failure {
    if timed_out(e) {
        return Failure::TimedOut;
    }
    Failure::Refused(format!("devtools: reading /json/list: {e}"))
}

/// The response head: a 200 status and its `Content-Length`.
fn head(reader: &mut impl BufRead) -> Result<usize, Failure> {
    let status = line(reader)?;
    if !status.starts_with("HTTP/1.1 200") {
        return Err(format!("devtools: /json/list answered `{status}`").into());
    }
    let mut length = None;
    loop {
        let line = line(reader)?;
        if line.is_empty() {
            let none = || {
                "devtools: /json/list had no Content-Length"
                    .to_owned()
                    .into()
            };
            return length.ok_or_else(none);
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            let parsed = value.trim().parse().map_err(|_| {
                Failure::Refused(format!(
                    "devtools: /json/list's Content-Length is `{}`",
                    value.trim()
                ))
            })?;
            length = Some(parsed);
        }
    }
}

fn line(reader: &mut impl BufRead) -> Result<String, Failure> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| read_failure(&e))?;
    Ok(line.trim_end().to_owned())
}
