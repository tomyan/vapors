use super::*;

fn tool(binary: &str, checksum: Option<&str>) -> Tool {
    Tool {
        crate_name: binary.into(),
        binary: binary.into(),
        version: "1".into(),
        checksum: checksum.map(str::to_owned),
    }
}

#[test]
fn a_tool_without_a_checksum_passes() {
    assert_eq!(verify(&tool("no-such-binary-xyz", None)), Ok(()));
}

#[test]
fn a_binary_matching_its_checksum_passes_and_another_is_refused() {
    // Given: sh's own hash
    let path = located("sh").expect("sh");
    let right = sha256(&path).expect("hash");

    // Then
    assert_eq!(verify(&tool("sh", Some(&right))), Ok(()));
    let refusal = verify(&tool("sh", Some(&"0".repeat(64)))).expect_err("refused");
    assert!(refusal.contains("not the 0000"), "{refusal}");
    assert!(refusal.contains("fix: "), "{refusal}");
}

#[test]
fn a_pinned_binary_not_on_path_is_refused() {
    let refusal = verify(&tool("no-such-binary-xyz", Some(&"0".repeat(64)))).expect_err("refused");
    assert!(refusal.contains("not on PATH"), "{refusal}");
}
