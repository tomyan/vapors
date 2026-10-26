//! `verify` against a server started here on 127.0.0.1: one that serves
//! as static-web-server does passes; one without compression fails.

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;

use super::*;
use crate::serve_verify_rules::expect_for;

/// A copy with index.html, a script it loads and two wasm files.
fn copy(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-verify-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("worker")).expect("dir");
    std::fs::write(
        dir.join("index.html"),
        r#"<script src="./browser.js?v=7"></script>"#,
    )
    .expect("index");
    std::fs::write(dir.join("touche-web-0123456789abcdef_bg.wasm"), "w").expect("wasm");
    std::fs::write(dir.join("worker/encode_worker_bg.wasm"), "w").expect("worker");
    dir
}

/// Serve every request with the headers `compressed` decides, forever.
fn server(compressed: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}/", listener.local_addr().expect("addr"));
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            let _ = BufReader::new(&stream).read_line(&mut line);
            let path = line
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .trim_start_matches('/')
                .split('?')
                .next()
                .unwrap_or("")
                .to_owned();
            let expect = expect_for(if path.is_empty() { "index.html" } else { &path });
            let encoding = if compressed {
                "Content-Encoding: br\r\n"
            } else {
                ""
            };
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\n{encoding}Vary: accept-encoding\r\nCache-Control: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                expect.content_type, expect.cache
            );
            let _ = (&stream).write_all(reply.as_bytes());
        }
    });
    base
}

#[test]
fn a_server_answering_as_static_web_server_does_passes_for_every_file() {
    // When
    let checked = verify(&server(true), &copy("good")).expect("passes");

    // Then: /, both wasm files at any depth, and the script
    assert_eq!(
        checked,
        "/ as br, touche-web-0123456789abcdef_bg.wasm as br, worker/encode_worker_bg.wasm as br, browser.js?v=7 as br"
    );
}

#[test]
fn a_server_without_compression_is_refused_naming_the_url() {
    let base = server(false);
    let refusal = verify(&base, &copy("plain")).expect_err("refused");
    assert!(
        refusal.starts_with(&base) && refusal.contains("not br or gzip"),
        "{refusal}"
    );
}

#[test]
fn a_copy_without_wasm_is_refused() {
    let dir = copy("nowasm");
    std::fs::remove_file(dir.join("touche-web-0123456789abcdef_bg.wasm")).expect("rm");
    std::fs::remove_file(dir.join("worker/encode_worker_bg.wasm")).expect("rm");
    assert!(
        verify(&server(true), &dir)
            .expect_err("no wasm")
            .contains("has no .wasm")
    );
}
