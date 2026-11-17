//! The LLC's line size is the highest-level data or unified cache's, as
//! sysfs gives it, or refused naming the path.

use std::path::Path;

use super::line_bytes_under;

fn cache(root: &Path, index: u32, level: &str, kind: &str, line: &str) {
    let dir = root.join(format!("index{index}"));
    std::fs::create_dir_all(&dir).expect("dir");
    for (name, text) in [
        ("level", level),
        ("type", kind),
        ("coherency_line_size", line),
    ] {
        std::fs::write(dir.join(name), format!("{text}\n")).expect("written");
    }
}

fn scratch(test: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(test)
        .tempdir()
        .expect("scratch")
}

#[test]
fn the_highest_level_data_or_unified_cache_gives_the_line() {
    // Given: an instruction cache at a higher level than the last unified
    let root = scratch("llc-highest");
    cache(root.path(), 0, "1", "Data", "64");
    cache(root.path(), 1, "2", "Unified", "64");
    cache(root.path(), 2, "3", "Unified", "128");
    cache(root.path(), 3, "4", "Instruction", "32");

    // Then
    assert_eq!(line_bytes_under(root.path()), Ok(128));
}

#[test]
fn an_unreadable_or_malformed_description_is_refused_naming_the_path() {
    // Given
    let root = scratch("llc-malformed");
    cache(root.path(), 0, "3", "Unified", "lots");

    // When
    let malformed = line_bytes_under(root.path()).expect_err("malformed");
    let missing = line_bytes_under(&root.path().join("absent")).expect_err("missing");

    // Then
    assert!(
        malformed.contains("coherency_line_size:1: `lots`"),
        "{malformed}"
    );
    assert!(missing.contains("absent"), "{missing}");
}
