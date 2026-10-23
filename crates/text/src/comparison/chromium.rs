//! Headless Chromium as the reference renderer: find it, screenshot a
//! page at a fixed size and scale, refuse anything but a PNG of that
//! size.

use std::fmt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use super::chromium_process::{DEADLINE, check_size, drain, finish, on_path, remove_stale};
use super::measure::Rgba;
use super::png_file::read_png;

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
pub struct Chromium {
    binary: PathBuf,
    /// How long one screenshot may take before Chromium is killed.
    deadline: Duration,
}

impl Chromium {
    /// `VAPORS_CHROMIUM`, else the first candidate on `PATH`.
    ///
    /// # Errors
    ///
    /// [`ChromiumError::NotFound`], naming what was tried and the fix.
    pub fn find() -> Result<Self, ChromiumError> {
        if let Some(binary) = std::env::var_os(CHROMIUM_ENV) {
            return Ok(Self {
                binary: PathBuf::from(binary),
                deadline: DEADLINE,
            });
        }
        CANDIDATES
            .iter()
            .find_map(|name| on_path(name))
            .map(|binary| Self {
                binary,
                deadline: DEADLINE,
            })
            .ok_or(ChromiumError::NotFound)
    }

    /// The binary's `--version` line, for labels.
    #[must_use]
    pub fn version(&self) -> String {
        Command::new(&self.binary)
            .arg("--version")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| self.binary.display().to_string())
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
        remove_stale(png)?;
        let mut child = self.spawn(html, png, width, height, scale)?;
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
        let mut child = self
            .command(width, height, scale)
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
        width: u32,
        height: u32,
        scale: u32,
    ) -> Result<Child, ChromiumError> {
        self.command(width, height, f64::from(scale))
            .arg(format!("--screenshot={}", png.display()))
            .arg(format!("file://{}", html.display()))
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| self.cannot_start(&e))
    }

    /// Headless Chromium at a fixed window and scale. It asks
    /// xdg-desktop-portal over the session bus before its first request;
    /// where the portal cannot start, D-Bus activation waits 120 s. With
    /// no session bus address it skips that.
    fn command(&self, width: u32, height: u32, scale: f64) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
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
            ])
            .stderr(Stdio::piped());
        command
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
