//! Each Chromium launch keeps its disk cache inside its own temp
//! directory: `XDG_CACHE_HOME` and `--disk-cache-dir` name a directory
//! under that launch's `TMPDIR`, which exists while it runs and is gone
//! after, however the launch ends. The caller's own `XDG_CACHE_HOME` is
//! never the cache.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::super::chromium::Chromium;
use super::super::chromium_tmp::contain_cache;

/// A "browser" that appends `TMPDIR|XDG_CACHE_HOME|exists|args` to
/// `record` and exits `code`.
fn recording(dir: &Path, record: &Path, code: i32) -> Chromium {
    let script = dir.join("chromium");
    let text = format!(
        "#!/bin/sh\n[ \"$1\" = --version ] && {{ echo 'Chromium 1'; exit 0; }}\n\
         [ -d \"$XDG_CACHE_HOME\" ] && there=there || there=absent\n\
         echo \"$TMPDIR|$XDG_CACHE_HOME|$there|$*\" >> {}\nexit {code}\n",
        record.display()
    );
    std::fs::write(&script, text).expect("script");
    let made = Command::new("chmod").arg("755").arg(&script).status();
    assert!(made.expect("chmod").success());
    Chromium {
        binary: script,
        deadline: Duration::from_secs(5),
        profiles: dir.join("profiles"),
    }
}

fn scratch(name: &str) -> tempfile::TempDir {
    let made = tempfile::Builder::new()
        .prefix(&format!("chromium-cache-{name}-"))
        .tempdir();
    made.expect("a scratch directory under the temp dir")
}

/// The one recorded launch: its cache dir is inside its TMPDIR, existed,
/// is named by `--disk-cache-dir`, and is gone now.
fn assert_contained(record: &Path) {
    let text = std::fs::read_to_string(record).expect("the browser ran");
    let line = text.lines().next().expect("one launch");
    let fields: Vec<&str> = line.splitn(4, '|').collect();
    let (tmp, cache) = (PathBuf::from(fields[0]), PathBuf::from(fields[1]));
    assert!(
        cache.starts_with(&tmp),
        "{} not in {}",
        cache.display(),
        tmp.display()
    );
    assert_ne!(cache, tmp);
    assert_eq!(
        fields[2],
        "there",
        "{} absent while it ran",
        cache.display()
    );
    let flag = format!("--disk-cache-dir={}", cache.display());
    assert!(
        fields[3].split_whitespace().any(|a| a == flag),
        "no {flag} in {}",
        fields[3]
    );
    assert!(
        !cache.exists() && !tmp.exists(),
        "{} is left",
        tmp.display()
    );
}

#[test]
fn a_screenshot_that_fails_keeps_its_cache_in_its_tmpdir_and_removes_it() {
    let dir = scratch("shot");
    let record = dir.path().join("record");
    let chromium = recording(dir.path(), &record, 1);
    let _ = chromium.screenshot(
        &dir.path().join("p.html"),
        &dir.path().join("p.png"),
        10,
        10,
        1,
    );
    assert_contained(&record);
}

#[test]
fn a_dom_dump_that_succeeds_keeps_its_cache_in_its_tmpdir_and_removes_it() {
    let dir = scratch("dom");
    let record = dir.path().join("record");
    let chromium = recording(dir.path(), &record, 0);
    chromium
        .dump_dom(&dir.path().join("p.html"), 10, 10, 1.0)
        .expect("a zero exit is a dump");
    assert_contained(&record);
}

#[test]
fn a_live_browser_keeps_its_cache_in_its_tmpdir_and_removes_it_on_drop() {
    let dir = scratch("live");
    let record = dir.path().join("record");
    let chromium = recording(dir.path(), &record, 0);
    let live = chromium
        .live(&dir.path().join("profile"), 10, 10, 1)
        .expect("starts");
    let started = Instant::now();
    while std::fs::read_to_string(&record)
        .unwrap_or_default()
        .is_empty()
        && started.elapsed() < Duration::from_secs(10)
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(live);
    assert_contained(&record);
}

#[test]
fn a_cache_that_cannot_be_made_is_refused_naming_it_and_never_touches_the_command() {
    let dir = scratch("refuse");
    let tmp = dir.path().join("missing");
    let mut command = Command::new("true");
    let refused = contain_cache(&mut command, &tmp).expect_err("no tmp, no cache");
    let message = refused.to_string();
    let cache = tmp.join("cache");
    assert!(message.contains(&cache.display().to_string()), "{message}");
    let under = format!("under {}", dir.path().display());
    assert!(message.ends_with(&under), "{message}");
    assert_eq!(command.get_args().count(), 0);
    assert_eq!(command.get_envs().count(), 0);
}
