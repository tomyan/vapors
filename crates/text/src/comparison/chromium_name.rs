//! Which browser a launch used, for labels and failures: on the runner
//! `chromium` and `google-chrome` are different browsers, each with its
//! own first-run cost (vapors #37), so every failure names the binary
//! and its version.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::chromium::{Chromium, ChromiumError};
use super::chromium_process::{drain, finish};

/// How long `--version` may take: it answers in under 0.1 s here. A
/// broken browser that hangs must not hang the message that names it.
const VERSION_DEADLINE: Duration = Duration::from_secs(10);

impl Chromium {
    /// The browser binary this launches.
    #[must_use]
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// The binary's `--version` line, or why there is none.
    #[must_use]
    pub fn version(&self) -> String {
        self.version_line()
            .unwrap_or_else(|why| format!("version unknown: {why}"))
    }

    fn version_line(&self) -> Result<String, String> {
        let mut child = Command::new(self.binary())
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdout = drain(child.stdout.take().ok_or("its stdout was not piped")?);
        let (_, text) = finish(&mut child, &stdout, VERSION_DEADLINE).map_err(|e| e.to_string())?;
        let line = text.trim().to_owned();
        if line.is_empty() {
            return Err("it printed nothing".to_owned());
        }
        Ok(line)
    }

    /// `error` with the browser's path and version first.
    pub(super) fn named(&self, error: ChromiumError) -> ChromiumError {
        match error {
            ChromiumError::Failed(message) => ChromiumError::Failed(format!(
                "{} ({}): {message}",
                self.binary().display(),
                self.version()
            )),
            ChromiumError::NotFound => ChromiumError::NotFound,
        }
    }
}
