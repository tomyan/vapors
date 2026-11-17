//! The API route end to end: a request through the harness server, the
//! policy and the budget, to a curl stand-in that keeps the body it was
//! handed, and back (review of #476-#478, L7; lead, L37).

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use crate::api_budget::Budget;
use crate::api_prices::Price;
use crate::api_server::{Routes, answered};
use crate::api_server_proxy::Proxy;
use crate::ux_secret::Secret;

/// A curl stand-in in `dir` that copies the body it is handed (the
/// `--data-binary @<file>` argument) to `dir/forwarded` and answers a
/// whole message.
fn keeping_curl(dir: &Path) -> Vec<String> {
    let script = dir.join("curl");
    let kept = dir.join("forwarded");
    std::fs::write(
        &script,
        format!(
            "for a; do case \"$a\" in @/*) cp \"${{a#@}}\" {};; esac; done\ncat > /dev/null\n\
             printf 'HTTP/1.1 200 OK\\r\\ncontent-type: application/json\\r\\n\\r\\n'\n\
             printf '{{\"model\":\"m\",\"usage\":{{\"input_tokens\":1,\"output_tokens\":1}}}}'\n",
            kept.display()
        ),
    )
    .expect("written");
    vec!["/bin/sh".into(), script.display().to_string()]
}

/// `body` posted to `/v1/messages` through the server; its answer, and
/// the bytes the stand-in was handed, if any.
fn posted(body: &str) -> (String, Option<Vec<u8>>) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let dollar = Price {
        input: 1.0,
        output: 1.0,
        cache_write: 1.0,
        cache_read: 1.0,
    };
    let budget = Budget::new(10.0, BTreeMap::from([("m".to_owned(), dollar)])).expect("a budget");
    let secret = Secret::new("sk-ant-a-key-long-enough-to-guard-0123456789".into()).expect("a key");
    let staging: PathBuf = dir.path().join("staging");
    std::fs::create_dir(&staging).expect("made");
    let proxy = Proxy::new(secret, budget, staging).through(keeping_curl(dir.path()));
    let routes = Routes {
        proxy: Some(proxy),
        ..Routes::default()
    };
    let request = format!(
        "POST /v1/messages HTTP/1.1\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    let (mut client, mut server) = UnixStream::pair().expect("a pair");
    client.write_all(request.as_bytes()).expect("sent");
    client.shutdown(Shutdown::Write).expect("shut");
    answered(&mut server, &routes);
    drop(server);
    let mut answer = String::new();
    let _ = client.read_to_string(&mut answer);
    (answer, std::fs::read(dir.path().join("forwarded")).ok())
}

#[test]
fn the_body_forwarded_is_the_parse_judged_not_the_bytes_sent() {
    // Given: a body with whitespace the parse does not keep
    let sent = r#"{ "model" : "m",  "max_tokens" : 1, "messages" : [ ] }"#;

    // When
    let (answer, forwarded) = posted(sent);

    // Then
    assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
    let forwarded = String::from_utf8(forwarded.expect("forwarded")).expect("text");
    assert_eq!(forwarded, r#"{"model":"m","max_tokens":1,"messages":[]}"#);
}

#[test]
fn a_body_with_a_repeated_key_is_refused_and_never_forwarded() {
    let (answer, forwarded) =
        posted(r#"{"model":"m","max_tokens":128000,"messages":[],"max_tokens":1}"#);
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert!(answer.contains("appears twice"), "{answer}");
    assert_eq!(forwarded, None);
}
