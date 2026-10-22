//! Headless Chromium as the reference renderer: find it, screenshot a
//! page at a fixed size and scale, refuse anything but a PNG of that
//! size.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::chromium_process::{check_size, on_path, read_all, remove_stale, wait_with_deadline};
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
            });
        }
        CANDIDATES
            .iter()
            .find_map(|name| on_path(name))
            .map(|binary| Self { binary })
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
        // Drained on its own thread so a chatty Chromium cannot fill the
        // pipe, block, and be reported as hung.
        let stderr = child
            .stderr
            .take()
            .map(|stream| std::thread::spawn(move || read_all(stream)));
        let status = wait_with_deadline(&mut child)?;
        let stderr = stderr.and_then(|t| t.join().ok()).unwrap_or_default();
        if !status.success() {
            return Err(ChromiumError::Failed(format!(
                "exited with {status}:\n{stderr}"
            )));
        }
        let image = read_png(png).map_err(ChromiumError::Failed)?;
        check_size(&image, png, [width * scale, height * scale])?;
        Ok(image)
    }

    fn spawn(
        &self,
        html: &Path,
        png: &Path,
        width: u32,
        height: u32,
        scale: u32,
    ) -> Result<Child, ChromiumError> {
        // Headless Chromium asks xdg-desktop-portal over the session bus
        // before its first request; where the portal cannot start, D-Bus
        // activation waits 120 s. With no session bus address it skips that.
        Command::new(&self.binary)
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .args([
                "--headless=new",
                "--no-sandbox",
                "--hide-scrollbars",
                &format!("--force-device-scale-factor={scale}"),
                // Fonts load after first paint; give the page settled time.
                "--virtual-time-budget=5000",
                &format!("--window-size={width},{height}"),
                &format!("--screenshot={}", png.display()),
                &format!("file://{}", html.display()),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                ChromiumError::Failed(format!("cannot start {}: {e}", self.binary.display()))
            })
    }
}

#[cfg(test)]
#[path = "chromium_tests.rs"]
mod tests;
