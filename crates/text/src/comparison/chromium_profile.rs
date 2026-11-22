//! Each launch's Chromium profile, in the automation cache and named for
//! the last UTC day it is kept.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::chromium::ChromiumError;

/// The automation cache's directory for Chromium profiles.
const DIR: &str = "chromium-headless";

/// Days a profile is kept after the UTC day it is made: the automation
/// cache's existing retention (8 days, chromium-cache-retention's `AGE`).
/// The daily retention job removes a profile on the day after its date,
/// once its scan finds no process using it.
const RETENTION_DAYS: u64 = 8;

const DAY_SECONDS: u64 = 86_400;

/// `$XDG_CACHE_HOME/chromium-headless`, or `$HOME/.cache/chromium-headless`
/// when `XDG_CACHE_HOME` is unset or not absolute (the XDG rule).
///
/// # Errors
///
/// [`ChromiumError::Failed`] when neither gives an absolute path.
pub(super) fn profile_root(
    xdg_cache_home: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, ChromiumError> {
    let absolute = |value: Option<OsString>| value.map(PathBuf::from).filter(|p| p.is_absolute());
    if let Some(cache) = absolute(xdg_cache_home) {
        return Ok(cache.join(DIR));
    }
    absolute(home)
        .map(|home| home.join(".cache").join(DIR))
        .ok_or_else(|| {
            ChromiumError::Failed(
                "no automation cache for its profile: neither XDG_CACHE_HOME nor HOME is an \
             absolute path. Fix: set HOME (or XDG_CACHE_HOME) to an absolute directory"
                    .to_owned(),
            )
        })
}

/// `expires-YYYY-MM-DD-`: the UTC day [`RETENTION_DAYS`] after `now`'s.
///
/// # Errors
///
/// [`ChromiumError::Failed`] when `now` is before 1970.
pub(super) fn expiry_prefix(now: SystemTime) -> Result<String, ChromiumError> {
    let since = now.duration_since(UNIX_EPOCH).map_err(|_| {
        ChromiumError::Failed(
            "the clock reads before 1970, so its profile has no expiry date. Fix: set the \
             system clock"
                .to_owned(),
        )
    })?;
    let (year, month, day) = civil_date(since.as_secs() / DAY_SECONDS + RETENTION_DAYS);
    Ok(format!("expires-{year:04}-{month:02}-{day:02}-"))
}

/// (year, month, day) of the day `days` after 1970-01-01, proleptic
/// Gregorian (H. Hinnant, "chrono-Compatible Low-Level Date Algorithms",
/// `civil_from_days`).
fn civil_date(days: u64) -> (u64, u64, u64) {
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// A new profile directory `root/expires-YYYY-MM-DD-<id>`, `root` made
/// first if missing. Removed when dropped; a kill leaves it for the
/// retention job, which needs no process to clean up after it.
///
/// # Errors
///
/// [`ChromiumError::Failed`] naming the directory that cannot be made.
pub(super) fn launch_profile(
    root: &Path,
    now: SystemTime,
) -> Result<tempfile::TempDir, ChromiumError> {
    let cannot = |e: std::io::Error| {
        ChromiumError::Failed(format!(
            "a profile for it under {}: {e}. Fix: make that directory writable, or free space",
            root.display()
        ))
    };
    std::fs::create_dir_all(root).map_err(cannot)?;
    tempfile::Builder::new()
        .prefix(&expiry_prefix(now)?)
        .tempdir_in(root)
        .map_err(cannot)
}

/// Remove a launch's profile now, saying so when it cannot be.
///
/// # Errors
///
/// [`ChromiumError::Failed`] naming the profile left behind.
pub(super) fn remove_profile(profile: tempfile::TempDir) -> Result<(), ChromiumError> {
    let path = profile.path().to_owned();
    profile.close().map_err(|e| {
        ChromiumError::Failed(format!(
            "its profile {} could not be removed: {e}. Fix: remove it by hand; the daily \
             retention job removes it after its expiry date otherwise",
            path.display()
        ))
    })
}

#[cfg(test)]
#[path = "chromium_profile_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chromium_profile_launch_tests.rs"]
mod launch_tests;
