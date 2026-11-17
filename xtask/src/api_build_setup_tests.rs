use std::os::unix::fs::PermissionsExt;

use super::key;

#[test]
fn a_key_file_others_can_read_is_refused_naming_the_fix() {
    // Given
    let dir = tempfile::tempdir().expect("a scratch directory");
    let file = dir.path().join("key");
    std::fs::write(&file, "sk-ant-a-key-long-enough-to-guard-0123456789").expect("written");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).expect("mode");

    // When
    let refused = key(&file).expect_err("refused");

    // Then
    assert!(refused.contains("is mode 644; fix: chmod 600"), "{refused}");
}

#[test]
fn a_private_key_file_is_read() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let file = dir.path().join("key");
    std::fs::write(&file, "sk-ant-a-key-long-enough-to-guard-0123456789\n").expect("written");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("mode");
    assert!(key(&file).is_ok());
}
