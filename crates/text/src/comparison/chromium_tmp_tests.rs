//! Each Chromium launch runs with a TMPDIR of its own, which exists while
//! it runs and is gone once the call returns or the live browser drops.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::super::chromium::Chromium;

/// A "browser" that appends its TMPDIR, and whether that directory
/// exists, to `record`, then exits 1. `--version` (asked to name the
/// browser in an error, and not a launch with a page) only answers.
fn recording(dir: &Path, record: &Path) -> Chromium {
    let script = dir.join("chromium");
    let text = format!(
        "#!/bin/sh\n[ \"$1\" = --version ] && {{ echo 'Chromium 1'; exit 0; }}\n\
         [ -d \"$TMPDIR\" ] && there=there || there=absent\necho \"$TMPDIR $there\" >> {}\nexit 1\n",
        record.display()
    );
    std::fs::write(&script, text).expect("script");
    let made = Command::new("chmod")
        .arg("755")
        .arg(&script)
        .status()
        .expect("chmod");
    assert!(made.success());
    Chromium {
        binary: script,
        deadline: Duration::from_secs(5),
    }
}

/// Each launch's TMPDIR, checked to have existed while it ran.
fn launches(record: &Path) -> Vec<PathBuf> {
    let text = std::fs::read_to_string(record).unwrap_or_default();
    text.lines()
        .map(|line| {
            let (tmp, there) = line.rsplit_once(' ').expect("two fields");
            assert_eq!(there, "there", "{tmp} did not exist while it ran");
            PathBuf::from(tmp)
        })
        .collect()
}

/// Two launches had TMPDIRs of their own under the temp dir, both gone.
fn assert_own_and_gone(tmps: &[PathBuf]) {
    assert_eq!(tmps.len(), 2, "{tmps:?}");
    assert_ne!(tmps[0], tmps[1]);
    for tmp in tmps {
        assert!(tmp.starts_with(std::env::temp_dir()), "{}", tmp.display());
        assert!(!tmp.exists(), "{} is left", tmp.display());
    }
}

fn scratch(name: &str) -> tempfile::TempDir {
    let made = tempfile::Builder::new()
        .prefix(&format!("chromium-tmp-{name}-"))
        .tempdir();
    made.expect("a scratch directory under the temp dir")
}

#[test]
fn each_screenshot_has_a_tmpdir_of_its_own_removed_after() {
    // Given
    let dir = scratch("shot");
    let record = dir.path().join("record");
    let chromium = recording(dir.path(), &record);

    // When
    for _ in 0..2 {
        let _ = chromium.screenshot(
            &dir.path().join("p.html"),
            &dir.path().join("p.png"),
            10,
            10,
            1,
        );
    }

    // Then
    assert_own_and_gone(&launches(&record));
}

#[test]
fn each_dom_dump_has_a_tmpdir_of_its_own_removed_after() {
    // Given
    let dir = scratch("dom");
    let record = dir.path().join("record");
    let chromium = recording(dir.path(), &record);

    // When
    for _ in 0..2 {
        let _ = chromium.dump_dom(&dir.path().join("p.html"), 10, 10, 1.0);
    }

    // Then
    assert_own_and_gone(&launches(&record));
}

#[test]
fn each_live_browser_has_a_tmpdir_of_its_own_removed_when_dropped() {
    // Given
    let dir = scratch("live");
    let record = dir.path().join("record");
    let chromium = recording(dir.path(), &record);

    // When: each started, seen to record, then dropped
    for n in 0..2 {
        let live = chromium
            .live(&dir.path().join(format!("profile{n}")), 10, 10, 1)
            .expect("starts");
        let started = Instant::now();
        while launches(&record).len() <= n && started.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(live);
    }

    // Then
    assert_own_and_gone(&launches(&record));
}
