//! Headless Chromium as the reference renderer: find it, screenshot a
//! page at a fixed size and scale, refuse anything but a PNG of that
//! size.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
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
        Command::new(&self.binary)
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

/// Delete an earlier screenshot so a stale image can never be read back.
fn remove_stale(png: &Path) -> Result<(), ChromiumError> {
    match std::fs::remove_file(png) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ChromiumError::Failed(format!(
            "cannot remove the previous {}: {e}",
            png.display()
        ))),
    }
}

fn check_size(image: &Rgba, png: &Path, wanted: [u32; 2]) -> Result<(), ChromiumError> {
    if [image.width, image.height] == wanted {
        return Ok(());
    }
    Err(ChromiumError::Failed(format!(
        "{} is {}x{}, wanted {}x{}. Fix: the window size or scale was not honoured; \
         check --force-device-scale-factor and that no display scaling applies",
        png.display(),
        image.width,
        image.height,
        wanted[0],
        wanted[1]
    )))
}

fn wait_with_deadline(child: &mut Child) -> Result<ExitStatus, ChromiumError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stale_screenshot_is_removed_and_a_missing_one_is_fine() {
        // Given
        let dir = std::env::temp_dir().join(format!("vapors-chromium-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let png = dir.join("stale.png");
        std::fs::write(&png, b"old").expect("write");

        // When
        remove_stale(&png).expect("removed");
        let again = remove_stale(&png);

        // Then
        assert!(!png.exists());
        assert_eq!(again, Ok(()));
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_screenshot_of_the_wrong_size_is_refused_naming_both_sizes() {
        // Given
        let image = Rgba {
            width: 10,
            height: 5,
            data: vec![0; 200],
        };

        // When
        let error = check_size(&image, Path::new("x.png"), [20, 10]).expect_err("wrong size");

        // Then
        let message = error.to_string();
        assert!(message.contains("10x5, wanted 20x10"), "{message}");
        assert!(message.contains("Fix:"), "{message}");
    }
}
