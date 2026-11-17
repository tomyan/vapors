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

/// `body` posted to the push route of `routes`; the server's answer.
fn push_asked(body: &str, routes: &Routes) -> String {
    let request = format!(
        "POST /api-build/push HTTP/1.1\r\ncontent-length: {}\r\n\r\n{body}",
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

/// Routes with a pusher whose paths are never reached: each case below
/// is refused before any git runs.
fn pushing() -> Routes {
    let builder = crate::api_paths::Builder::new("b", std::path::Path::new("/nonexistent"))
        .expect("a builder");
    let host = crate::api_sandbox::Host {
        xtask: "/nonexistent/xtask".into(),
        claude: "/nonexistent/claude".into(),
        objects: "/nonexistent/objects".into(),
        render_nodes: Vec::new(),
        advisories: Vec::new(),
    };
    let pusher = crate::api_push::Pusher::new("/nonexistent".into(), builder, host, Vec::new());
    Routes {
        push: Some(pusher),
        ..Routes::default()
    }
}

#[test]
fn a_push_without_a_title_and_a_body_or_not_json_is_refused() {
    for body in [r#"{"title":"t"}"#, r#"{"body":"b"}"#, "not json"] {
        let answer = push_asked(body, &pushing());
        assert!(answer.starts_with("HTTP/1.1 403"), "{body}: {answer}");
    }
}

#[test]
fn a_push_on_a_run_with_no_pusher_is_refused() {
    let answer = push_asked(r#"{"title":"t","body":"b"}"#, &Routes::default());
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
}

#[test]
fn a_push_whose_pr_text_mentions_someone_is_refused_before_anything_runs() {
    let answer = push_asked(r#"{"title":"t","body":"thanks @someone"}"#, &pushing());
    assert!(answer.starts_with("HTTP/1.1 409"), "{answer}");
    assert!(answer.contains("mentions"), "{answer}");
}
