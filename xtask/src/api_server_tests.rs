use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

use super::{Refused, answered};

/// `request` sent, the server's answer to it, and what it refused.
fn exchanged(request: &[u8]) -> (String, Vec<String>) {
    let (mut client, mut server) = UnixStream::pair().expect("a socket pair");
    let mut sender = client.try_clone().expect("a second handle");
    let request = request.to_vec();
    // sent beside the server, as a request past the socket's buffer blocks
    let sent = std::thread::spawn(move || {
        let _ = sender.write_all(&request); // the server may close before the end
        let _ = sender.shutdown(Shutdown::Write);
    });
    let refused = Refused::default();
    answered(&mut server, &refused);
    drop(server);
    sent.join().expect("the sender");
    let mut answer = Vec::new();
    // a server that closes on unread bytes resets the connection after
    // its answer; the answer is what came before
    let _ = client.read_to_end(&mut answer);
    let answer = String::from_utf8_lossy(&answer).into_owned();
    let log = refused.lock().expect("the log").clone();
    (answer, log)
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
    assert_eq!(refused, ["GET /etc/passwd HTTP/1.1"]);
}

#[test]
fn a_line_past_the_limit_is_refused_without_reading_it_all() {
    let long = vec![b'a'; 64 * 1024];
    let (answer, refused) = exchanged(&long);
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert_eq!(refused, ["a request line past 8192 bytes, or cut off"]);
}
