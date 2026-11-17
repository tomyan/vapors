use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

use std::sync::atomic::Ordering;

use super::{Routes, answered};

/// `request` sent, the server's answer to it, and what it refused.
fn exchanged(request: &[u8]) -> (String, Vec<String>) {
    let (answer, routes) = exchanged_with(request);
    let (log, _) = routes.refused.kept();
    (answer, log)
}

/// `request` sent, the server's answer to it, and its routes after.
fn exchanged_with(request: &[u8]) -> (String, Routes) {
    let (mut client, mut server) = UnixStream::pair().expect("a socket pair");
    let mut sender = client.try_clone().expect("a second handle");
    let request = request.to_vec();
    // sent beside the server, as a request past the socket's buffer blocks
    let sent = std::thread::spawn(move || {
        let _ = sender.write_all(&request); // the server may close before the end
        let _ = sender.shutdown(Shutdown::Write);
    });
    let routes = Routes::default();
    answered(&mut server, &routes);
    drop(server);
    sent.join().expect("the sender");
    let mut answer = Vec::new();
    // a server that closes on unread bytes resets the connection after
    // its answer; the answer is what came before
    let _ = client.read_to_end(&mut answer);
    (String::from_utf8_lossy(&answer).into_owned(), routes)
}

#[test]
fn the_way_in_probe_is_answered_pong() {
    let (answer, refused) = exchanged(b"GET /api-build/ping HTTP/1.1\r\n\r\n");
    assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
    assert!(answer.ends_with("pong"), "{answer}");
    assert_eq!(refused, Vec::<String>::new());
}

#[test]
fn a_request_with_no_route_is_refused_and_logged() {
    let (answer, refused) = exchanged(b"GET /etc/passwd HTTP/1.1\r\n\r\n");
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert_eq!(refused, ["GET /etc/passwd"]);
}

#[test]
fn a_line_past_the_limit_is_refused_without_reading_it_all() {
    let long = vec![b'a'; 64 * 1024];
    let (answer, refused) = exchanged(&long);
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert_eq!(refused, ["a request head line past 8192 bytes or cut off"]);
}

#[test]
fn the_clis_connectivity_check_is_answered_here_and_counted() {
    // When
    let (answer, routes) = exchanged_with(b"HEAD /api/hello HTTP/1.1\r\n\r\n");

    // Then: 200, nothing forwarded (there is no proxy), counted
    assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
    assert_eq!(routes.hellos.load(Ordering::Relaxed), 1);
}

#[test]
fn the_api_is_refused_on_a_run_with_no_key() {
    let (answer, refused) = exchanged(b"POST /v1/messages HTTP/1.1\r\ncontent-length: 2\r\n\r\n{}");
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert_eq!(refused, ["POST /v1/messages"]);
}

#[test]
fn every_connection_has_its_writes_bounded() {
    // Given
    let (mut client, mut server) = UnixStream::pair().expect("a socket pair");
    client
        .write_all(b"GET /api-build/ping HTTP/1.1\r\n\r\n")
        .expect("sent");

    // When
    answered(&mut server, &Routes::default());

    // Then: a session that stops reading holds a write for at most this
    assert_eq!(
        server.write_timeout().ok().flatten(),
        Some(super::WRITE_DEADLINE)
    );
}
