use super::*;

#[test]
fn a_time_reads_as_utc_to_the_second() {
    // 2026-10-26 12:34:56 UTC
    assert_eq!(utc(1_793_018_096), "2026-10-26T12:34:56Z");
    assert_eq!(utc(0), "1970-01-01T00:00:00Z");
    // A leap day.
    assert_eq!(utc(1_709_208_000), "2024-02-29T12:00:00Z");
}

#[test]
fn each_line_is_appended_with_its_time() {
    // Given
    let path = std::env::temp_dir().join(format!("xtask-queue-log-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let log = Log { path: path.clone() };

    // When
    log.line("#68 merged at aaa").expect("written");
    log.line("#70 has no review clear").expect("written");

    // Then
    let text = std::fs::read_to_string(&path).expect("read");
    let _ = std::fs::remove_file(&path);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].ends_with("Z #68 merged at aaa"), "{text}");
    assert!(lines[1].ends_with("Z #70 has no review clear"), "{text}");
}
