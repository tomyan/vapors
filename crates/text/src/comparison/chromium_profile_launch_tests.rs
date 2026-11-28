//! Each launch runs Chromium with `--user-data-dir` naming its own
//! expiry-named profile under the automation cache, which exists while it
//! runs and is removed once the call returns, however the launch ends.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::super::chromium::Chromium;

/// A "browser" that appends `profile|exists|args` to `record`, runs
/// `extra` (shell), prints `<html></html>` and exits `code`.
fn recording(dir: &Path, record: &Path, code: i32, extra: &str) -> Chromium {
    let script = dir.join("chromium");
    let text = format!(
        "#!/bin/sh\n[ \"$1\" = --version ] && {{ echo 'Chromium 1'; exit 0; }}\n\
         p=none; for a in \"$@\"; do case $a in --user-data-dir=*) p=${{a#--user-data-dir=}};; esac; done\n\
         [ -d \"$p\" ] && there=there || there=absent\n\
         echo \"$p|$there|$*\" >> {}\n{extra}\necho '<html></html>'\nexit {code}\n",
        record.display()
    );
    std::fs::write(&script, text).expect("script");
    let made = Command::new("chmod").arg("755").arg(&script).status();
    assert!(made.expect("chmod").success());
    Chromium {
        binary: script,
        deadline: Duration::from_secs(5),
        profiles: dir.join("cache").join("chromium-headless"),
    }
}

/// The one recorded launch's profile: under `root`, named
/// `expires-YYYY-MM-DD-<id>`, present while it ran.
fn launched_profile(record: &Path, root: &Path) -> PathBuf {
    let text = std::fs::read_to_string(record).expect("the browser ran");
    let line = text.lines().next().expect("one launch");
    let fields: Vec<&str> = line.splitn(3, '|').collect();
    let profile = PathBuf::from(fields[0]);
    assert_eq!(profile.parent(), Some(root), "{line}");
    let name = profile
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    let date = name.strip_prefix("expires-").expect("expires- prefix");
    let digits: Vec<bool> = date.chars().take(10).map(|c| c.is_ascii_digit()).collect();
    assert_eq!(
        digits,
        [true, true, true, true, false, true, true, false, true, true],
        "{name}"
    );
    let id = date
        .get(10..)
        .and_then(|rest| rest.strip_prefix('-'))
        .unwrap_or("");
    assert!(
        id.len() >= 6 && id.chars().all(|c| c.is_ascii_alphanumeric()),
        "{name}"
    );
    assert_eq!(
        fields[1],
        "there",
        "{} absent while it ran",
        profile.display()
    );
    profile
}

#[test]
fn a_launch_that_succeeds_runs_in_its_expiry_profile_and_removes_it() {
    let dir = tempfile::tempdir().expect("scratch");
    let record = dir.path().join("record");
    let chromium = recording(dir.path(), &record, 0, "");
    chromium
        .dump_dom(&dir.path().join("p.html"), 100, 50, 1.0)
        .expect("dumped");
    let profile = launched_profile(&record, &chromium.profiles);
    assert!(!profile.exists(), "{} is left", profile.display());
}

#[test]
fn a_launch_that_fails_runs_in_its_expiry_profile_and_removes_it() {
    let dir = tempfile::tempdir().expect("scratch");
    let record = dir.path().join("record");
    let chromium = recording(dir.path(), &record, 1, "");
    let shot = chromium.screenshot(
        &dir.path().join("p.html"),
        &dir.path().join("p.png"),
        10,
        10,
        1,
    );
    assert!(shot.is_err());
    let profile = launched_profile(&record, &chromium.profiles);
    assert!(!profile.exists(), "{} is left", profile.display());
}

#[test]
fn a_profile_that_cannot_be_removed_after_a_good_launch_is_refused_naming_it() {
    let dir = tempfile::tempdir().expect("scratch");
    let record = dir.path().join("record");
    let lock = "mkdir \"$p/locked\" && touch \"$p/locked/f\" && chmod 500 \"$p/locked\"";
    let chromium = recording(dir.path(), &record, 0, lock);
    let result = chromium.dump_dom(&dir.path().join("p.html"), 100, 50, 1.0);
    let profile = launched_profile(&record, &chromium.profiles);
    let unlock = Command::new("chmod")
        .arg("755")
        .arg(profile.join("locked"))
        .status();
    assert!(unlock.expect("chmod").success());
    let error = result.expect_err("refused").to_string();
    assert!(error.contains(&profile.display().to_string()), "{error}");
}

#[test]
fn a_profile_that_cannot_be_removed_after_a_failed_launch_is_named_with_the_failure() {
    let dir = tempfile::tempdir().expect("scratch");
    let record = dir.path().join("record");
    let lock = "mkdir \"$p/locked\" && touch \"$p/locked/f\" && chmod 500 \"$p/locked\"";
    let chromium = recording(dir.path(), &record, 1, lock);
    let shot = chromium.screenshot(
        &dir.path().join("p.html"),
        &dir.path().join("p.png"),
        10,
        10,
        1,
    );
    let profile = launched_profile(&record, &chromium.profiles);
    let unlock = Command::new("chmod")
        .arg("755")
        .arg(profile.join("locked"))
        .status();
    assert!(unlock.expect("chmod").success());
    let error = shot.expect_err("refused").to_string();
    assert!(error.contains("exited with"), "{error}");
    assert!(error.contains(&profile.display().to_string()), "{error}");
}

#[test]
fn a_profile_root_that_cannot_be_made_is_refused_before_launching() {
    let dir = tempfile::tempdir().expect("scratch");
    let record = dir.path().join("record");
    let mut chromium = recording(dir.path(), &record, 0, "");
    std::fs::write(dir.path().join("file"), "").expect("file");
    chromium.profiles = dir.path().join("file").join("chromium-headless");
    let error = chromium.dump_dom(&dir.path().join("p.html"), 100, 50, 1.0);
    assert!(
        error
            .expect_err("refused")
            .to_string()
            .contains("chromium-headless")
    );
    assert!(!record.exists(), "the browser ran");
}
