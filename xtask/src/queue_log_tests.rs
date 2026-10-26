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

#[test]
fn a_log_from_an_earlier_day_is_rotated_and_old_ones_deleted() {
    // Given: today's log written on 2026-10-20, and rotations of 10-01 and 10-18
    let dir = std::env::temp_dir().join(format!("xtask-queue-rotate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let log = dir.join("touche.log");
    std::fs::write(&log, "2026-10-20T10:00:00Z a line\n").expect("log");
    std::fs::write(dir.join("touche.log.2026-10-01"), "old").expect("old");
    std::fs::write(dir.join("touche.log.2026-10-18"), "recent").expect("recent");

    // When: rotated on 2026-10-22
    rotate(&log, "2026-10-22").expect("rotated");

    // Then
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("read")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(names, ["touche.log.2026-10-18", "touche.log.2026-10-20"]);
}

#[test]
fn a_log_from_today_is_kept_and_a_missing_one_is_fine() {
    let dir = std::env::temp_dir().join(format!("xtask-queue-today-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let log = dir.join("vapors.log");
    rotate(&log, "2026-10-22").expect("nothing to rotate");
    std::fs::write(&log, "2026-10-22T01:00:00Z a line\n").expect("log");
    rotate(&log, "2026-10-22").expect("kept");
    assert!(log.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
