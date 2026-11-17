use std::path::Path;

use super::{arguments, passed_head};

#[test]
fn curls_arguments_hold_no_header_and_no_key() {
    // When
    let args = arguments(
        "POST",
        "/v1/messages?beta=true",
        Path::new("/private/1.json"),
    );

    // Then: headers come from stdin (`@-`) only
    let headers: Vec<&String> = args
        .windows(2)
        .filter(|w| w[0] == "--header")
        .map(|w| &w[1])
        .collect();
    assert_eq!(headers, ["@-"]);
    assert!(
        !args.iter().any(|arg| arg.contains("x-api-key")),
        "{args:?}"
    );
    assert_eq!(
        args.last().map(String::as_str),
        Some("https://api.anthropic.com/v1/messages?beta=true")
    );
}

#[test]
fn the_response_head_is_passed_with_its_framing_replaced_by_a_close() {
    // Given
    let response = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\
                    content-length: 5\r\nconnection: keep-alive\r\n\r\nbody";
    let mut reader = response.as_bytes();
    let mut client = Vec::new();

    // When
    let status = passed_head(&mut reader, &mut client).expect("a head");

    // Then
    assert_eq!(status, 200);
    let head = String::from_utf8(client).expect("text");
    assert_eq!(
        head,
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n"
    );
    assert_eq!(reader, b"body");
}

#[test]
fn a_response_that_is_not_http_is_refused() {
    let refused = passed_head(&mut "curl: (6) no host\n".as_bytes(), &mut Vec::new());
    assert!(
        refused
            .expect_err("refused")
            .starts_with("a response that does not start")
    );
}

mod forwarded {
    use std::io::Read;
    use std::os::unix::net::UnixStream;

    use super::super::{Staged, forward_by};
    use crate::api_http::Request;
    use crate::api_usage::Counted;
    use crate::ux_secret::Secret;

    const HEAD: &str = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n";
    const START: &str = "data: {\"type\":\"message_start\",\"message\":{\"model\":\"m\",\"usage\":{\"input_tokens\":3,\"output_tokens\":1}}}\n";
    const STOP: &str = "data: {\"type\":\"message_stop\"}\n";

    /// A curl stand-in in `dir` that drains its stdin, prints `output`,
    /// holds on for `hold` seconds, and exits with `code`. It ignores
    /// SIGPIPE, so a reader that goes away does not end it: only a kill
    /// does.
    fn fake_curl(dir: &std::path::Path, output: &str, (code, hold): (i32, u32)) -> String {
        let program = dir.join("curl");
        let printed = output
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        std::fs::write(
            &program,
            format!(
                "#!/bin/sh\ntrap '' PIPE\ncat > /dev/null\nprintf \"{printed}\"\nsleep {hold}\nexit {code}\n"
            ),
        )
        .expect("written");
        std::fs::set_permissions(
            &program,
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .expect("mode");
        program.display().to_string()
    }

    /// A request forwarded through `curl`, the session reading all of it
    /// (`reads`) or closing after the first read; what it counted, and
    /// whether the staged body was left.
    fn counted(output: &str, (code, hold): (i32, u32), reads: bool) -> (Counted, bool) {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let curl = fake_curl(dir.path(), output, (code, hold));
        let secret =
            Secret::new("sk-ant-a-key-long-enough-to-guard-0123456789".into()).expect("a key");
        let request = Request {
            method: "POST".into(),
            target: "/v1/messages".into(),
            headers: Vec::new(),
            body: b"{}".to_vec(),
        };
        let staged_at = dir.path().join("1.json");
        let (mut session, mut ours) = UnixStream::pair().expect("a pair");
        let reader = std::thread::spawn(move || {
            if reads {
                let mut all = Vec::new();
                let _ = session.read_to_end(&mut all);
            } else {
                let mut first = [0; 256]; // the head and a little more, then gone
                let _ = session.read(&mut first);
            }
        });
        let staged = Staged::new(staged_at.clone(), &request.body).expect("staged");
        let forwarded =
            forward_by(&curl, &request, (&secret, &staged), &mut ours).expect("forwarded");
        drop((staged, ours));
        reader.join().expect("the reader");
        (forwarded.counted, staged_at.exists())
    }

    #[test]
    fn a_whole_stream_counts_and_leaves_no_staged_body() {
        let (counted, left) = counted(&format!("{HEAD}{START}{STOP}"), (0, 0), true);
        assert!(
            matches!(counted, Counted::Whole(_, ref model) if model == "m"),
            "{counted:?}"
        );
        assert!(!left);
    }

    #[test]
    fn a_transfer_curl_ends_with_an_error_is_cut_though_the_stream_looked_whole() {
        let (counted, _) = counted(&format!("{HEAD}{START}{STOP}"), (18, 0), true);
        assert!(
            matches!(counted, Counted::Cut(ref why) if why.contains("curl ended")),
            "{counted:?}"
        );
    }

    #[test]
    fn a_session_that_stops_reading_ends_curl_at_once_is_cut_and_leaves_no_staged_body() {
        // Given: a curl that would hold on for 30 s after its output
        let long = "data: {\"type\":\"ping\"}\n".repeat(20_000);
        let started = std::time::Instant::now();

        // When
        let (counted, left) = counted(&format!("{HEAD}{START}{long}{STOP}"), (0, 30), false);

        // Then: killed, not waited for (review of #476-#478, M1)
        assert!(started.elapsed().as_secs() < 10, "{:?}", started.elapsed());
        assert!(matches!(counted, Counted::Cut(_)), "{counted:?}");
        assert!(!left);
    }
}

#[test]
fn curl_never_expands_the_target_or_goes_through_a_proxy() {
    let args = arguments("POST", "/v1/messages", Path::new("/p/1.json"));
    assert!(args.iter().any(|arg| arg == "--globoff"), "{args:?}");
    assert!(args.windows(2).any(|w| w == ["--noproxy", "*"]), "{args:?}");
}
