//! A Chromium that stays up, for the `DevTools` protocol: headless, at a
//! fixed window and scale, on a profile of its own, with
//! `--remote-debugging-port=0`, so `vapors-devtools` reads the port from
//! the profile and drives the page. Its stderr goes to a file beside the
//! profile, so an early exit says why ([`Live::alive`]). It is stopped,
//! with what it started, and its profile removed, when dropped.

use std::fs::File;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::chromium::{Chromium, ChromiumError};
use super::chromium_process::kill_group;

/// A running headless Chromium.
pub struct Live {
    child: Child,
    profile: PathBuf,
    stderr: PathBuf,
}

impl Chromium {
    /// Start Chromium on `about:blank` at `width` by `height` CSS pixels
    /// and `scale`, on `profile`, which must be absent or empty: a profile
    /// in use would keep a stale `DevToolsActivePort`.
    ///
    /// # Errors
    ///
    /// [`ChromiumError::Failed`] when the profile has contents, or
    /// Chromium cannot start.
    pub fn live(
        &self,
        profile: &Path,
        width: u32,
        height: u32,
        scale: u32,
    ) -> Result<Live, ChromiumError> {
        fresh(profile)?;
        let stderr = profile.with_extension("stderr.log");
        let log = File::create(&stderr).map_err(|e| failed(&stderr, &e))?;
        let child = self
            .live_command(profile, width, height, scale)
            .stderr(log)
            .spawn()
            .map_err(|e| {
                ChromiumError::Failed(format!("cannot start {}: {e}", self.binary.display()))
            })?;
        Ok(Live {
            child,
            profile: profile.to_path_buf(),
            stderr,
        })
    }

    fn live_command(&self, profile: &Path, width: u32, height: u32, scale: u32) -> Command {
        let mut command = Command::new(&self.binary);
        command
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
            .stdout(Stdio::null());
        command
    }
}

impl Live {
    /// The profile, where Chromium writes `DevToolsActivePort`.
    #[must_use]
    pub fn profile(&self) -> &Path {
        &self.profile
    }

    /// Whether Chromium is still running; when it has exited, why, with
    /// the end of its stderr.
    ///
    /// # Errors
    /// Chromium has exited, or cannot be asked.
    pub fn alive(&mut self) -> Result<(), String> {
        match self.child.try_wait() {
            Ok(None) => Ok(()),
            Ok(Some(status)) => Err(format!(
                "chromium exited with {status}; its stderr ended:\n{}",
                tail(&self.stderr)
            )),
            Err(e) => Err(format!("chromium: waiting on it: {e}")),
        }
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        if let Err(why) = kill_group(&mut self.child) {
            eprintln!("chromium: stopping it: {why}; it may still be running");
        }
        let _ = std::fs::remove_dir_all(&self.profile);
        let _ = std::fs::remove_file(&self.stderr);
    }
}

/// `profile` absent or empty, then made; one with contents is refused.
fn fresh(profile: &Path) -> Result<(), ChromiumError> {
    let has_contents = std::fs::read_dir(profile).is_ok_and(|mut entries| entries.next().is_some());
    if has_contents {
        return Err(ChromiumError::Failed(format!(
            "{} is not empty; a live Chromium needs a fresh profile",
            profile.display()
        )));
    }
    std::fs::create_dir_all(profile).map_err(|e| failed(profile, &e))
}

fn failed(path: &Path, error: &std::io::Error) -> ChromiumError {
    ChromiumError::Failed(format!("{}: {error}", path.display()))
}

/// The last 20 lines of `path`.
fn tail(path: &Path) -> String {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| format!("({}: {e})", path.display()));
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(20)..].join("\n")
}

#[cfg(test)]
#[path = "chromium_live_tests.rs"]
mod tests;
