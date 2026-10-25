//! A Chromium that stays up, for the `DevTools` protocol: headless, at a
//! fixed window and scale, on a fresh profile, with
//! `--remote-debugging-port=0`, so `vapors-devtools` reads the port from
//! the profile and drives the page. It is stopped, with what it started,
//! when dropped.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::chromium::{Chromium, ChromiumError};
use super::chromium_process::kill_group;

/// A running headless Chromium.
pub struct Live {
    child: Child,
    profile: PathBuf,
}

impl Chromium {
    /// Start Chromium on `about:blank` at `width` by `height` CSS pixels
    /// and `scale`, with `profile` emptied first.
    ///
    /// # Errors
    ///
    /// [`ChromiumError::Failed`] when the profile cannot be emptied or
    /// Chromium cannot start.
    pub fn live(
        &self,
        profile: &Path,
        width: u32,
        height: u32,
        scale: u32,
    ) -> Result<Live, ChromiumError> {
        empty(profile)?;
        let child = Command::new(&self.binary)
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .process_group(0)
            .args([
                "--headless=new",
                "--no-sandbox",
                "--hide-scrollbars",
                "--remote-debugging-port=0",
            ])
            .arg(format!("--force-device-scale-factor={scale}"))
            .arg(format!("--window-size={width},{height}"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("about:blank")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                ChromiumError::Failed(format!("cannot start {}: {e}", self.binary.display()))
            })?;
        Ok(Live {
            child,
            profile: profile.to_path_buf(),
        })
    }
}

impl Live {
    /// The profile, where Chromium writes `DevToolsActivePort`.
    #[must_use]
    pub fn profile(&self) -> &Path {
        &self.profile
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = kill_group(&mut self.child);
    }
}

/// `profile` removed and made again; a removal that fails for another
/// reason than its absence is refused, since a stale `DevToolsActivePort`
/// would name an old port.
fn empty(profile: &Path) -> Result<(), ChromiumError> {
    match std::fs::remove_dir_all(profile) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(ChromiumError::Failed(format!("{}: {e}", profile.display()))),
    }
    std::fs::create_dir_all(profile)
        .map_err(|e| ChromiumError::Failed(format!("{}: {e}", profile.display())))
}
