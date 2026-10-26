use super::*;

const SHA: &str = "4b39efe44e995bfb1a19517079046490ccc9b4e9907bf924ec46bef3197be56b";

#[test]
fn a_checksum_is_kept_on_its_tool() {
    let text = format!(
        "[tools]\nstatic-web-server = \"2.44.1\"\n[checksums]\nstatic-web-server = \"{SHA}\"\n"
    );
    let tools = parse(&text).expect("valid");
    assert_eq!(tools[0].checksum.as_deref(), Some(SHA));
}

#[test]
fn a_checksum_that_is_not_sha256_hex_is_refused_naming_its_line() {
    let text = "[tools]\nsws = \"1\"\n[checksums]\nsws = \"ABC\"\n";
    let error = parse(text).expect_err("refused");
    assert_eq!(error.line, 4);
    assert!(error.reason.contains("64 lower-case hex"), "{error}");
}

#[test]
fn a_checksum_for_a_tool_not_listed_is_refused() {
    let text = format!("[tools]\nbacon = \"3\"\n[checksums]\nsws = \"{SHA}\"\n");
    let error = parse(&text).expect_err("refused");
    assert!(error.reason.contains("not in [tools]"), "{error}");
}
