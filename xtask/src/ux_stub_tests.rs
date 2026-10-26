//! A local stand-in for the Responses API, for the tests of the call and
//! the live run: it answers one request and reports what it was sent.

use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::sync::mpsc;

/// Answer one request with `status` and `reply`; send back the
/// Authorization header and the body it carried.
pub fn stand_in(status: &'static str, reply: &str) -> (String, mpsc::Receiver<(String, String)>) {
    stand_in_many(vec![(status, reply.to_owned())])
}

/// Answer one request per entry of `replies`, in order, each on its own
/// connection; send back each request's Authorization header and body.
pub fn stand_in_many(
    replies: Vec<(&'static str, String)>,
) -> (String, mpsc::Receiver<(String, String)>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bound");
    let url = format!(
        "http://{}/v1/responses",
        listener.local_addr().expect("address")
    );
    let (send, seen) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, reply) in replies {
            let (stream, _) = listener.accept().expect("accepted");
            send.send(answer_one(stream, status, &reply)).expect("sent");
        }
    });
    (url, seen)
}

fn answer_one(stream: std::net::TcpStream, status: &str, reply: &str) -> (String, String) {
    let mut reader = BufReader::new(stream);
    let (mut auth, mut length) = (String::new(), 0usize);
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("read");
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            length = v.trim().parse().expect("length");
        }
        if lower.starts_with("authorization:") {
            auth = line.trim_end().to_owned();
        }
        if line == "\r\n" {
            break;
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).expect("body");
    let answer = format!(
        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
        reply.len()
    );
    std::io::Write::write_all(reader.get_mut(), answer.as_bytes()).expect("answered");
    (auth, String::from_utf8(body).expect("utf-8"))
}
