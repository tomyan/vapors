//! Finding the page of a Chromium started with `--remote-debugging-port=0`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use super::*;

const SHORT: Duration = Duration::from_millis(500);

#[allow(
    clippy::unnecessary_wraps,
    reason = "it stands in for Live::alive, which can fail"
)]
fn alive() -> Result<(), String> {
    Ok(())
}

fn profile(test: &str, port_file: Option<&str>) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("devtools-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    if let Some(text) = port_file {
        std::fs::write(dir.join("DevToolsActivePort"), text).expect("file");
    }
    dir
}

#[test]
fn the_port_is_read_once_both_lines_are_written() {
    let dir = profile("whole", Some("9222\n/devtools/browser/abc"));
    assert_eq!(port(&dir, SHORT, &mut alive), Ok(9222));
}

#[test]
fn a_half_written_port_file_is_not_trusted_and_the_wait_fails_naming_it() {
    // Given: "92" of "9222", no second line yet
    let dir = profile("half", Some("92"));

    // When
    let refusal = port(&dir, SHORT, &mut alive).expect_err("not trusted");

    // Then
    assert!(refusal.contains("DevToolsActivePort"), "{refusal}");
}

/// An HTTP server answering `/json/list` with `bodies`, one per request.
fn targets(bodies: Vec<&'static str>) -> u16 {
    targets_with("200 OK", bodies)
}

fn targets_with(status: &'static str, bodies: Vec<&'static str>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("address").port();
    thread::spawn(move || {
        for body in bodies {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(body.as_bytes());
            thread::sleep(Duration::from_millis(50));
        }
    });
    port
}

#[test]
fn the_target_list_is_polled_until_a_page_appears() {
    // Given: no targets yet, then a page
    let port = targets(vec![
        "[]",
        r#"[{"type":"page","webSocketDebuggerUrl":"ws://127.0.0.1:1/devtools/page/A"}]"#,
    ]);

    // When / Then
    assert_eq!(
        page_socket(port, SHORT * 4),
        Ok("ws://127.0.0.1:1/devtools/page/A".to_owned())
    );
}

#[test]
fn no_page_by_the_deadline_is_refused_quoting_the_last_list() {
    let port = targets(vec!["[]"; 40]);
    let refusal = page_socket(port, SHORT).expect_err("no page");
    assert!(refusal.contains("no page target"), "{refusal}");
    assert!(refusal.contains("[]"), "{refusal}");
}

#[test]
fn the_host_sent_carries_the_port_as_chromium_builds_its_urls_from_it() {
    // Given: a server that, as Chromium does, puts the request's Host in the URL
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("address").port();
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut request = [0u8; 1024];
        let read = stream.read(&mut request).unwrap_or(0);
        let text = String::from_utf8_lossy(&request[..read]).into_owned();
        let host = text
            .lines()
            .find_map(|l| l.strip_prefix("Host: "))
            .unwrap_or("")
            .to_owned();
        let body =
            format!(r#"[{{"type":"page","webSocketDebuggerUrl":"ws://{host}/devtools/page/A"}}]"#);
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(body.as_bytes());
        thread::sleep(Duration::from_millis(200));
    });

    // When / Then
    assert_eq!(
        page_socket(port, SHORT),
        Ok(format!("ws://127.0.0.1:{port}/devtools/page/A"))
    );
}

#[test]
fn a_port_file_that_cannot_be_read_or_parsed_is_refused_at_once() {
    // Given: a directory where the file should be, and a port that is not a number
    let unreadable = profile("unreadable", None);
    std::fs::create_dir_all(unreadable.join("DevToolsActivePort")).expect("dir");
    let garbled = profile("garbled", Some("abc\n/devtools/browser/x"));

    for (dir, needle) in [(unreadable, "DevToolsActivePort"), (garbled, "abc")] {
        // When
        let started = Instant::now();
        let refusal = port(&dir, SHORT, &mut alive).expect_err(needle);

        // Then: at once, not at the deadline
        assert!(started.elapsed() < SHORT / 2, "{:?}", started.elapsed());
        assert!(refusal.contains(needle), "{refusal}");
    }
}

#[test]
fn a_chromium_that_has_exited_stops_the_wait_with_its_reason() {
    // Given
    let dir = profile("exited", None);

    // When
    let refusal = port(&dir, SHORT, &mut || {
        Err("chromium exited with status 1: bad flag".into())
    })
    .expect_err("exited");

    // Then
    assert!(refusal.contains("bad flag"), "{refusal}");
}

#[test]
fn a_target_list_of_another_shape_is_refused_with_its_own_reason() {
    for (status, body, needle) in [
        ("500 Internal Server Error", "[]", "500"),
        ("200 OK", "{}", "not a list"),
        ("200 OK", r#"[{"type":"page"}]"#, "no webSocketDebuggerUrl"),
    ] {
        let port = targets_with(status, vec![body; 20]);
        let refusal = page_socket(port, SHORT).expect_err(needle);
        assert!(refusal.contains(needle), "wanted {needle} in: {refusal}");
    }
}
