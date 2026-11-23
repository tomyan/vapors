//! Headless Chromium as the reference renderer: find it, screenshot a
//! page at a fixed size and scale, refuse anything but a PNG of that
//! size.

use std::fmt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::chromium_process::{DEADLINE, drain, finish, on_path};
use super::chromium_profile::{in_profile, profile_root};
use super::chromium_tmp::{contain_cache, launch_tmp};
use super::measure::Rgba;

/// Environment variable naming the browser binary, overriding the search.
pub const CHROMIUM_ENV: &str = "VAPORS_CHROMIUM";

/// Binary names tried on `PATH`, in order.
const CANDIDATES: [&str; 4] = [
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
];

/// Why Chromium could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromiumError {
    /// No binary found.
    NotFound,
    /// It ran and failed, or produced the wrong thing.
    Failed(String),
}

impl fmt::Display for ChromiumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(
                f,
                "no Chromium found: tried {} on PATH. Fix: install chromium, or set {CHROMIUM_ENV} \
                 to the browser binary.",
                CANDIDATES.join(", ")
            ),
            Self::Failed(message) => write!(f, "chromium: {message}"),
        }
    }
}

impl std::error::Error for ChromiumError {}

/// A browser binary.
#[derive(Clone)]
pub struct Chromium {
    pub(super) binary: PathBuf,
    /// How long one screenshot may take before Chromium is killed.
    pub(super) deadline: Duration,
    /// Where each launch's expiry-named profile is made.
    pub(super) profiles: PathBuf,
}

impl Chromium {
    /// `VAPORS_CHROMIUM`, else the first candidate on `PATH`. Its
    /// profiles go in the automation cache ([`profile_root`]).
    ///
    /// # Errors
    ///
    /// [`ChromiumError::NotFound`], naming what was tried and the fix;
    /// [`ChromiumError::Failed`] when there is no automation cache.
    pub fn find() -> Result<Self, ChromiumError> {
        let binary = match std::env::var_os(CHROMIUM_ENV) {
            Some(binary) => PathBuf::from(binary),
            None => CANDIDATES
                .iter()
                .find_map(|name| on_path(name))
                .ok_or(ChromiumError::NotFound)?,
        };
        let profiles = profile_root(std::env::var_os("XDG_CACHE_HOME"), std::env::var_os("HOME"))?;
        Ok(Self {
            binary,
            deadline: DEADLINE,
            profiles,
        })
    }

    /// Screenshot `html` at `width` by `height` CSS pixels at `scale`
    /// device pixels per CSS pixel into `png`, and read it back: one
    /// screenshot in a [`Browser`](super::Browser) of its own, closed after it. A test
    /// taking more than one holds a [`Chromium::browser`] instead.
    ///
    /// # Errors
    ///
    /// As [`Browser::screenshot`](super::Browser::screenshot), and
    /// [`Browser::close`](super::Browser::close)'s failure.
    pub fn screenshot(
        &self,
        html: &Path,
        png: &Path,
        width: u32,
        height: u32,
        scale: u32,
    ) -> Result<Rgba, ChromiumError> {
        let mut browser = self.browser();
        let shot = browser.screenshot(html, png, width, height, scale);
        match (shot, browser.close()) {
            (shot, Ok(())) => shot,
            (Ok(_), Err(closing)) => Err(self.named(closing)),
            (Err(failure), Err(closing)) => {
                Err(ChromiumError::Failed(format!("{failure}\nand {closing}")))
            }
        }
    }

    /// The DOM of `html` at `width` by `height` CSS pixels and device
    /// scale `scale`, after its scripts and fonts have settled, as
    /// `--dump-dom` prints it.
    ///
    /// # Errors
    ///
    /// [`ChromiumError::Failed`] when it exits non-zero or hangs past the
    /// deadline.
    pub fn dump_dom(
        &self,
        html: &Path,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Result<String, ChromiumError> {
        let tmp = launch_tmp()?;
        in_profile(&self.profiles, std::time::SystemTime::now(), |profile| {
            self.dump_in(html, [tmp.path(), profile], [width, height], scale)
        })
    }

    /// One `--dump-dom` with `dirs` as its `[TMPDIR, profile]`.
    fn dump_in(
        &self,
        html: &Path,
        dirs: [&Path; 2],
        [width, height]: [u32; 2],
        scale: f64,
    ) -> Result<String, ChromiumError> {
        let mut child = self
            .command(dirs, width, height, scale)?
            .arg("--dump-dom")
            .arg(format!("file://{}", html.display()))
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| self.cannot_start(&e))?;
        let stdout = drain(child.stdout.take().ok_or_else(|| not_piped("stdout"))?);
        let stderr = drain(child.stderr.take().ok_or_else(|| not_piped("stderr"))?);
        let (status, stderr) = finish(&mut child, &stderr, self.deadline)?;
        let stdout = stdout
            .recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| {
                ChromiumError::Failed("its stdout was still open 2 s after it exited".to_owned())
            })?;
        if !status.success() {
            return Err(ChromiumError::Failed(format!(
                "exited with {status}:\n{stderr}"
            )));
        }
        Ok(stdout)
    }

    /// Headless Chromium at a fixed window and scale. It asks
    /// xdg-desktop-portal over the session bus before its first request;
    /// where the portal cannot start, D-Bus activation waits 120 s. With
    /// no session bus address it skips that. `[tmp, profile]`: `tmp` is its
    /// `TMPDIR` ([`launch_tmp`]) and holds its disk cache ([`contain_cache`]);
    /// `profile` is its `--user-data-dir` ([`launch_profile`]), so it makes
    /// no profile of its own under `~/.config/chromium-headless`.
    fn command(
        &self,
        [tmp, profile]: [&Path; 2],
        width: u32,
        height: u32,
        scale: f64,
    ) -> Result<Command, ChromiumError> {
        let mut command = Command::new(&self.binary);
        command
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .env("TMPDIR", tmp)
            // Its own process group, so the deadline kills what it starts.
            .process_group(0)
            .args([
                "--headless=new",
                "--no-sandbox",
                "--hide-scrollbars",
                &format!("--force-device-scale-factor={scale}"),
                // Fonts load after first paint; give the page settled time.
                "--virtual-time-budget=5000",
                &format!("--window-size={width},{height}"),
                &format!("--user-data-dir={}", profile.display()),
            ])
            .stderr(Stdio::piped());
        contain_cache(&mut command, tmp)?;
        Ok(command)
    }

    fn cannot_start(&self, error: &std::io::Error) -> ChromiumError {
        ChromiumError::Failed(format!("cannot start {}: {error}", self.binary.display()))
    }
}

fn not_piped(stream: &str) -> ChromiumError {
    ChromiumError::Failed(format!("its {stream} was not piped"))
}

#[cfg(test)]
#[path = "chromium_tests.rs"]
mod tests;
