//! Headless Chromium as the reference renderer: find it, screenshot a
//! page at a fixed size, refuse anything but a PNG of that size.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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

/// Chromium here starts and screenshots a page in about 1 s; a cold CI
/// runner was measured at under 10 s. 60 s is the ceiling before the run
/// is declared hung.
const DEADLINE: Duration = Duration::from_secs(60);

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

    /// Screenshot `html` at `width` by `height` CSS pixels at scale 1 into
    /// `png`, and read it back.
    ///
    /// # Errors
    ///
    /// [`ChromiumError::Failed`] when it exits non-zero, hangs past the
    /// deadline, or the PNG is not the requested size.
    pub fn screenshot(
        &self,
        html: &Path,
        png: &Path,
        width: u32,
        height: u32,
    ) -> Result<Rgba, ChromiumError> {
        let url = format!("file://{}", html.display());
        let mut child = Command::new(&self.binary)
            .args([
                "--headless=new",
                "--no-sandbox",
                "--hide-scrollbars",
                "--force-device-scale-factor=1",
                // Fonts load after first paint; give the page settled time.
                "--virtual-time-budget=5000",
                &format!("--window-size={width},{height}"),
                &format!("--screenshot={}", png.display()),
                &url,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                ChromiumError::Failed(format!("cannot start {}: {e}", self.binary.display()))
            })?;
        let status = wait_with_deadline(&mut child)?;
        if !status.success() {
            let stderr = child.stderr.take().map(read_all).unwrap_or_default();
            return Err(ChromiumError::Failed(format!(
                "exited with {status}:\n{stderr}"
            )));
        }
        let image = read_png(png).map_err(ChromiumError::Failed)?;
        if (image.width, image.height) != (width, height) {
            return Err(ChromiumError::Failed(format!(
                "{} is {}x{}, wanted {width}x{height}. Fix: the window size was not honoured; \
                 check --force-device-scale-factor and that no display scaling applies",
                png.display(),
                image.width,
                image.height
            )));
        }
        Ok(image)
    }
}

fn wait_with_deadline(
    child: &mut std::process::Child,
) -> Result<std::process::ExitStatus, ChromiumError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| ChromiumError::Failed(format!("waiting: {e}")))?
        {
            return Ok(status);
        }
        if started.elapsed() > DEADLINE {
            let _ = child.kill();
            return Err(ChromiumError::Failed(format!(
                "did not finish within {}s; killed",
                DEADLINE.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn read_all(mut stream: impl std::io::Read) -> String {
    let mut text = String::new();
    let _ = stream.read_to_string(&mut text);
    text
}

fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}
