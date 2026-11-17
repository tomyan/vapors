use super::escaped_to;

#[test]
fn an_escape_sequence_the_session_writes_to_stderr_comes_out_escaped() {
    // Given: OSC 52, which would set the lead's clipboard, on the
    // session's stderr
    let written = b"compiling\n\x1b]52;c;cm0gLXJm\x07 done\n";

    // When
    let mut out = Vec::new();
    escaped_to(&written[..], &mut out);

    // Then: no raw ESC or BEL reaches the terminal
    let out = String::from_utf8(out).expect("text");
    assert!(!out.bytes().any(|b| b == 0x1b || b == 0x07), "{out:?}");
    assert_eq!(
        out,
        "session: compiling\nsession: \\u{1b}]52;c;cm0gLXJm\\u{7} done\n"
    );
}
