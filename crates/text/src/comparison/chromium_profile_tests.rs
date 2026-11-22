//! A launch's profile is named `expires-YYYY-MM-DD-<id>` for the last UTC
//! day it is kept, eight days after the day it is made, and lives under
//! the automation cache's `chromium-headless`.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{expiry_prefix, launch_profile, profile_root};

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn the_expiry_date_is_the_utc_day_eight_days_after_the_launch() {
    // 2026-11-22T23:59:59Z, 2026-11-23T00:00:00Z, 1970-01-01T00:00:00Z,
    // 2026-12-28T12:00:00Z (into the next year), 2028-02-21T00:00:00Z (leap day).
    let cases = [
        (1_795_391_999, "expires-2026-11-30-"),
        (1_795_392_000, "expires-2026-12-01-"),
        (0, "expires-1970-01-09-"),
        (1_798_459_200, "expires-2027-01-05-"),
        (1_834_704_000, "expires-2028-02-29-"),
    ];
    for (seconds, expected) in cases {
        assert_eq!(
            expiry_prefix(at(seconds)).as_deref(),
            Ok(expected),
            "at {seconds} s"
        );
    }
}

#[test]
fn a_clock_before_1970_is_refused_naming_it() {
    let before = UNIX_EPOCH - Duration::from_secs(1);
    let error = expiry_prefix(before).expect_err("refused").to_string();
    assert!(error.contains("before 1970"), "{error}");
}

#[test]
fn the_root_is_chromium_headless_under_xdg_cache_home() {
    let root = profile_root(
        Some(OsString::from("/x/cache")),
        Some(OsString::from("/home/u")),
    );
    assert_eq!(root, Ok(PathBuf::from("/x/cache/chromium-headless")));
}

#[test]
fn without_an_absolute_xdg_cache_home_the_root_is_under_home() {
    for xdg in [
        None,
        Some(OsString::from("relative/cache")),
        Some(OsString::new()),
    ] {
        let root = profile_root(xdg.clone(), Some(OsString::from("/home/u")));
        assert_eq!(
            root,
            Ok(PathBuf::from("/home/u/.cache/chromium-headless")),
            "{xdg:?}"
        );
    }
}

#[test]
fn with_neither_an_absolute_xdg_cache_home_nor_home_the_root_is_refused_naming_both() {
    let error = profile_root(None, Some(OsString::from("rel")))
        .expect_err("refused")
        .to_string();
    assert!(
        error.contains("XDG_CACHE_HOME") && error.contains("HOME"),
        "{error}"
    );
}

#[test]
fn a_profile_is_made_under_the_root_with_the_expiry_prefix_creating_the_root() {
    let scratch = tempfile::tempdir().expect("scratch");
    let root = scratch.path().join("cache").join("chromium-headless");
    let profile = launch_profile(&root, at(1_795_391_999)).expect("made");
    let name = profile
        .path()
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    assert_eq!(profile.path().parent(), Some(root.as_path()));
    let id = name.strip_prefix("expires-2026-11-30-").expect("prefix");
    assert!(
        id.len() >= 6 && id.chars().all(|c| c.is_ascii_alphanumeric()),
        "{name}"
    );
    assert!(profile.path().is_dir());
}

#[test]
fn a_root_that_cannot_be_made_is_refused_naming_it() {
    let scratch = tempfile::tempdir().expect("scratch");
    let file = scratch.path().join("file");
    std::fs::write(&file, "").expect("file");
    let root = file.join("chromium-headless");
    let error = launch_profile(&root, at(0))
        .expect_err("refused")
        .to_string();
    assert!(error.contains(&root.display().to_string()), "{error}");
}
