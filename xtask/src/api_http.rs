//! The one HTTP shape the harness server reads (`api_server.rs`): a
//! request line, headers, and a body of `content-length` bytes. A request
//! of any other shape (a chunked body, a header line that is not
//! `name: value`, a line past [`LINE_LIMIT`], more than [`HEADER_LIMIT`]
//! headers) is refused, naming what was not understood. The caller bounds
//! the time it takes (`api_server.rs`).

use std::io::{BufRead, Read};

/// The longest line a request's head may hold: the CLI's are under 1 KB.
const LINE_LIMIT: usize = 8 * 1024;

/// The most header lines a request may have: the CLI sends about 15.
const HEADER_LIMIT: usize = 100;

/// The most a request's body may hold: the Messages API's own limit on a
/// request (32 MB), so nothing the API would take is refused and nothing
/// larger is held.
pub const BODY_LIMIT: usize = 32 * 1024 * 1024;

/// One request.
#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    /// The path and query, as sent.
    pub target: String,
    /// Each header's name, lowercased, and value.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// The path, without the query.
    pub fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or_default()
    }

    /// Header `name`'s value (`name` lowercase).
    pub fn header(&self, name: &str) -> Option<&str> {
        let found = self.headers.iter().find(|(header, _)| header == name);
        found.map(|(_, value)| value.as_str())
    }
}

/// The request `reader` holds, head and body: for the tests, as the
/// server reads the two apart.
///
/// # Errors
///
/// A request this shape does not cover, or the reader failing.
#[cfg(test)]
pub fn read(reader: &mut impl BufRead) -> Result<Request, String> {
    let mut request = read_head(reader)?;
    read_body(reader, &mut request)?;
    Ok(request)
}

/// The request line and headers `reader` holds; the body is left to
/// [`read_body`], so a caller can refuse before reading it.
///
/// # Errors
///
/// A request line other than `METHOD target HTTP/1.1`, a header name that
/// is not a token, a control character in a header value, or the caps
/// passed, each named (review of #476-#478, M8).
pub fn read_head(reader: &mut impl BufRead) -> Result<Request, String> {
    let line = head_line(reader)?;
    let parts: Vec<&str> = line.split(' ').collect();
    let [method, target, "HTTP/1.1"] = parts[..] else {
        return Err(format!(
            "a request line that is not `METHOD target HTTP/1.1`: {line:?}"
        ));
    };
    if !is_token(method) || target.chars().any(char::is_control) {
        return Err(format!(
            "a request line with a method or target this does not read: {line:?}"
        ));
    }
    Ok(Request {
        method: method.to_owned(),
        target: target.to_owned(),
        headers: headers(reader)?,
        body: Vec::new(),
    })
}

/// Whether `text` is an HTTP token: the characters a method or header
/// name may hold.
fn is_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c))
}

fn head_line(reader: &mut impl BufRead) -> Result<String, String> {
    let mut line = String::new();
    let read = reader
        .take(LINE_LIMIT as u64)
        .read_line(&mut line)
        .map_err(|e| format!("reading the request: {e}"))?;
    if read == 0 || !line.ends_with('\n') {
        return Err(format!(
            "a request head line past {LINE_LIMIT} bytes or cut off"
        ));
    }
    Ok(line.trim_end().to_owned())
}

fn headers(reader: &mut impl BufRead) -> Result<Vec<(String, String)>, String> {
    let mut headers = Vec::new();
    loop {
        if headers.len() > HEADER_LIMIT {
            return Err(format!("a request with more than {HEADER_LIMIT} headers"));
        }
        let line = head_line(reader)?;
        if line.is_empty() {
            return Ok(headers);
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(format!("a header line that is not `name: value`: {line:?}"));
        };
        let value = value.trim_matches([' ', '\t']);
        if !is_token(name) || value.chars().any(|c| c.is_control() && c != '\t') {
            return Err(format!(
                "a header with a name that is not a token or a control character in its value: {line:?}"
            ));
        }
        headers.push((name.to_ascii_lowercase(), value.to_owned()));
    }
}

/// `request`'s body, of its content-length, read from `reader`.
///
/// # Errors
///
/// A chunked body, a length that is not a number or past [`BODY_LIMIT`],
/// or fewer bytes than the length.
pub fn read_body(reader: &mut impl BufRead, request: &mut Request) -> Result<(), String> {
    request.body = body(reader, request)?;
    Ok(())
}

fn body(reader: &mut impl BufRead, request: &Request) -> Result<Vec<u8>, String> {
    if request.header("transfer-encoding").is_some() {
        return Err(
            "a chunked request body: the harness reads bodies of content-length bytes".into(),
        );
    }
    let length = match request.header("content-length") {
        None => return Ok(Vec::new()),
        Some(length) => length
            .parse::<usize>()
            .map_err(|_| format!("a content-length that is not a number: {length:?}"))?,
    };
    if length > BODY_LIMIT {
        return Err(format!(
            "a body of {length} bytes, past the harness's {BODY_LIMIT}"
        ));
    }
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .map_err(|e| format!("reading the {length}-byte body: {e}"))?;
    Ok(body)
}

#[cfg(test)]
#[path = "api_http_tests.rs"]
mod tests;
