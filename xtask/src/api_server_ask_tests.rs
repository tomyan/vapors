use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

use crate::api_ask::Asks;
use crate::api_server::{Routes, answered};

/// `body` posted to the ask route; the server's answer.
fn asked(body: &str, routes: &Routes) -> String {
    let request = format!(
        "POST /api-build/ask HTTP/1.1\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    let (mut client, mut server) = UnixStream::pair().expect("a pair");
    client.write_all(request.as_bytes()).expect("sent");
    client.shutdown(Shutdown::Write).expect("shut");
    answered(&mut server, routes);
    drop(server);
    let mut answer = String::new();
    let _ = client.read_to_string(&mut answer);
    answer
}

#[test]
fn an_ask_with_no_question_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let asks = Asks::for_run(dir.path(), "run-1").expect("a run directory");
    let run = asks.dir.clone();
    let routes = Routes {
        asks: Some(asks),
        ..Routes::default()
    };
    let answer = asked(r#"{"question":"  "}"#, &routes);
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert!(answer.contains("no question"), "{answer}");
    assert_eq!(std::fs::read_dir(run).expect("read").count(), 0);
}

#[test]
fn an_ask_on_a_run_with_no_questions_directory_is_refused() {
    let answer = asked(r#"{"question":"which?"}"#, &Routes::default());
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
}
