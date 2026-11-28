//! Headless Chromium as the reference renderer: find it, screenshot a
//! page at a fixed size and scale, refuse anything but a PNG of that
//! size.

use std::fmt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use super::chromium_process::{DEADLINE, check_size, drain, finish, remove_stale};
use super::chromium_profile::{in_profile, profile_root};
use super::chromium_tmp::{contain_cache, launch_tmp};
use super::measure::Rgba;
use super::png_file::read_png;

/// Environment variable naming the browser binary, overriding the search.
pub const CHROMIUM_ENV: &str = "VAPORS_CHROMIUM";

/// Why Chromium could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromiumError {
    /// The pinned build is not installed at this path, and no override.
    NotFound(PathBuf),
    /// No override, and `HOME` is not set, so the pinned build's path
    /// cannot be resolved.
    NoHome,
    /// It ran and failed, or produced the wrong thing.
    Failed(String),
}

impl fmt::Display for ChromiumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(pinned) => write!(
                f,
                "the pinned Chromium is not installed at {}. Fix: cargo xtask setup (it installs \
                 [downloads] chromium from .config/tools.toml), or set {CHROMIUM_ENV} to a browser \
                 binary for a diagnosis by hand.",
                pinned.display()
            ),
            Self::NoHome => write!(
                f,
                "HOME is not set, so the pinned Chromium's path (~/.local/share/factory/...) \
                 cannot be resolved. Fix: set HOME to the home `cargo xtask setup` installed \
                 into, or set {CHROMIUM_ENV} to a browser binary for a diagnosis by hand."
            ),
            Self::Failed(message) => write!(f, "chromium: {message}"),
        }
    }
}

impl std::error::Error for ChromiumError {}

/// A browser binary.
pub struct Chromium {
    pub(super) binary: PathBuf,
    /// How long one screenshot may take before Chromium is killed.
    pub(super) deadline: Duration,
    /// Where each launch's expiry-named profile is made.
    pub(super) profiles: PathBuf,
}

impl Chromium {
    /// `VAPORS_CHROMIUM`, else the pinned build (`chromium_pin.rs`), the
    /// same binary on every machine. No browser on `PATH` is used. Its
    /// profiles go in the automation cache ([`profile_root`]).
    ///
    /// # Errors
    ///
    /// [`ChromiumError::NotFound`] when the pinned build is not installed,
    /// naming its path and the fix; [`ChromiumError::NoHome`] when `HOME`
    /// is unset and there is no override; [`ChromiumError::Failed`] when
    /// there is no automation cache.
    pub fn find() -> Result<Self, ChromiumError> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(chosen) = std::env::var_os(CHROMIUM_ENV) {
            // a relaxed guard, declared where it runs: the override skips the
            // pin, for a diagnosis by hand; said on every use so a run that
            // inherits it is not mistaken for one on the pinned build
            eprintln!(
                "chromium: {CHROMIUM_ENV}={} overrides the pinned build",
                PathBuf::from(chosen).display()
            );
        }
        let binary = super::chromium_pin::resolve(
            std::env::var_os(CHROMIUM_ENV),
            home.as_deref(),
            Path::is_file,
        )?;
        let profiles = profile_root(std::env::var_os("XDG_CACHE_HOME"), std::env::var_os("HOME"))?;
        Ok(Self {
            binary,
            deadline: DEADLINE,
            profiles,
        })
    }

    /// Screenshot `html` at `width` by `height` CSS pixels at `scale`
    /// device pixels per CSS pixel into `png`, and read it back.
    ///
    /// # Errors
    ///
    /// [`ChromiumError::Failed`] when it exits non-zero, hangs past the
    /// deadline, a stale `png` cannot be removed first, or the PNG is not
    /// the requested size.
    pub fn screenshot(
        &self,
        html: &Path,
        png: &Path,
        width: u32,
        height: u32,
        scale: u32,
    ) -> Result<Rgba, ChromiumError> {
        self.shoot(html, png, [width, height], scale)
            .map_err(|error| self.named(error))
    }

    fn shoot(
        &self,
        html: &Path,
        png: &Path,
        size: [u32; 2],
        scale: u32,
    ) -> Result<Rgba, ChromiumError> {
        remove_stale(png)?;
        let tmp = launch_tmp()?;
        in_profile(&self.profiles, std::time::SystemTime::now(), |profile| {
            self.shoot_in(html, png, [tmp.path(), profile], size, scale)
        })
    }

    /// One screenshot with `dirs` as its `[TMPDIR, profile]`.
    fn shoot_in(
        &self,
        html: &Path,
        png: &Path,
        dirs: [&Path; 2],
        size: [u32; 2],
        scale: u32,
    ) -> Result<Rgba, ChromiumError> {
        let [width, height] = size;
        let mut child = self.spawn(html, png, dirs, size, scale)?;
        let stderr = drain(
            child
                .stderr
                .take()
                .ok_or_else(|| ChromiumError::Failed("its stderr was not piped".to_owned()))?,
        );
        let (status, stderr) = finish(&mut child, &stderr, self.deadline)?;
        if !status.success() {
            return Err(ChromiumError::Failed(format!(
                "exited with {status}:\n{stderr}"
            )));
        }
        let image = read_png(png).map_err(ChromiumError::Failed)?;
        check_size(&image, png, [width * scale, height * scale])?;
        Ok(image)
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

    fn spawn(
        &self,
        html: &Path,
        png: &Path,
        dirs: [&Path; 2],
        [width, height]: [u32; 2],
        scale: u32,
    ) -> Result<Child, ChromiumError> {
        self.command(dirs, width, height, f64::from(scale))?
            .arg(format!("--screenshot={}", png.display()))
            .arg(format!("file://{}", html.display()))
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| self.cannot_start(&e))
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
