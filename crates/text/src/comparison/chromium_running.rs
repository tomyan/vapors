//! One browser kept running for a test: headless Chromium at one device
//! scale with `--remote-debugging-port=0`, on an expiry-named profile
//! ([`launch_profile`]) and a `TMPDIR` of its own ([`launch_tmp`]) that
//! holds its disk cache and its stderr log, so nothing but the profile
//! directory lands in the automation cache.
//!
//! It runs in the test's process group (no `process_group(0)`), so
//! nextest's SIGTERM and SIGKILL of a timed-out test reach it. Stopping
//! it kills its processes by profile ([`kill_holding`]), never a group,
//! since the group is the test's. It is stopped, and its profile
//! removed, by [`Process::stop`] or when dropped.

use std::fs::File;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Instant, SystemTime};

use vapors_devtools::Session;

use super::chromium::{Chromium, ChromiumError};
use super::chromium_pids::kill_holding;
use super::chromium_profile::{launch_profile, remove_profile};
use super::chromium_tmp::{contain_cache, launch_tmp};

/// The started browser's processes, profile and `TMPDIR`.
pub(super) struct Process {
    child: Child,
    profile: Option<tempfile::TempDir>,
    tmp: tempfile::TempDir,
}

/// A browser whose `DevTools` socket is connected.
pub(super) struct Running {
    pub(super) scale: u32,
    pub(super) port: u16,
    pub(super) session: Session,
    pub(super) process: Process,
}

impl Running {
    /// Start Chromium at `size` CSS pixels and `scale`, and connect to it,
    /// all by `until`.
    ///
    /// # Errors
    /// It cannot start, exits, or has no socket by `until`; with the end
    /// of its stderr.
    pub(super) fn start(
        chromium: &Chromium,
        size: [u32; 2],
        scale: u32,
        until: Instant,
    ) -> Result<Self, ChromiumError> {
        let mut process = Process::spawn(chromium, size, scale)?;
        let connected = process.connect(until);
        let (port, session) = connected.map_err(|why| process.failed(&why))?;
        Ok(Self {
            scale,
            port,
            session,
            process,
        })
    }
}

impl Process {
    fn spawn(
        chromium: &Chromium,
        [width, height]: [u32; 2],
        scale: u32,
    ) -> Result<Self, ChromiumError> {
        let tmp = launch_tmp()?;
        let profile = launch_profile(&chromium.profiles, SystemTime::now())?;
        let log = tmp.path().join("stderr.log");
        let stderr = File::create(&log)
            .map_err(|e| ChromiumError::Failed(format!("{}: {e}", log.display())))?;
        let mut command = Command::new(&chromium.binary);
        command
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .env("TMPDIR", tmp.path())
            .args([
                "--headless=new",
                "--no-sandbox",
                "--hide-scrollbars",
                &format!("--force-device-scale-factor={scale}"),
                &format!("--window-size={width},{height}"),
                &format!("--user-data-dir={}", profile.path().display()),
                "--remote-debugging-port=0",
                "about:blank",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr);
        contain_cache(&mut command, tmp.path())?;
        let child = command.spawn().map_err(|e| {
            ChromiumError::Failed(format!("cannot start {}: {e}", chromium.binary.display()))
        })?;
        Ok(Self {
            child,
            profile: Some(profile),
            tmp,
        })
    }

    /// The port and a session on the browser target, by `until`.
    fn connect(&mut self, until: Instant) -> Result<(u16, Session), String> {
        let profile = self
            .profile
            .as_ref()
            .ok_or("its profile was already removed")?
            .path()
            .to_path_buf();
        let left = until.saturating_duration_since(Instant::now());
        let child = &mut self.child;
        let port = vapors_devtools::port(&profile, left, &mut || exited(child))?;
        let path = browser_path(&profile)?;
        let left = until.saturating_duration_since(Instant::now());
        let session = Session::connect(&format!("ws://127.0.0.1:{port}{path}"), left)?;
        Ok((port, session))
    }

    /// Whether it is still running; when it has exited, why.
    pub(super) fn state(&mut self) -> String {
        exited(&mut self.child).map_or_else(|why| why, |()| "it was still running".to_owned())
    }

    /// `why` with its state and the end of its stderr; it is stopped.
    pub(super) fn failed(&mut self, why: &str) -> ChromiumError {
        let state = self.state();
        let stderr = tail(&self.tmp.path().join("stderr.log"));
        let stopped = self
            .stop()
            .err()
            .map_or_else(String::new, |e| format!("\nand stopping it: {e}"));
        ChromiumError::Failed(format!(
            "{why} ({state}); its stderr ended:\n{stderr}{stopped}"
        ))
    }

    /// Kill its processes, reap it, and remove its profile.
    ///
    /// # Errors
    /// A process outlives the kill, or the profile cannot be removed.
    pub(super) fn stop(&mut self) -> Result<(), ChromiumError> {
        let Some(profile) = self.profile.take() else {
            return Ok(());
        };
        let killed = kill_holding(profile.path());
        let _ = self.child.kill();
        let _ = self.child.wait();
        let removed = remove_profile(profile);
        killed.map_err(ChromiumError::Failed).and(removed)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Err(why) = self.stop() {
            eprintln!("{why}");
        }
    }
}

fn exited(child: &mut Child) -> Result<(), String> {
    match child.try_wait() {
        Ok(None) => Ok(()),
        Ok(Some(status)) => Err(format!("it exited with {status}")),
        Err(e) => Err(format!("waiting on it: {e}")),
    }
}

/// The browser target's path, the second line of `DevToolsActivePort`.
fn browser_path(profile: &Path) -> Result<String, String> {
    let file = profile.join("DevToolsActivePort");
    let text = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    match text.lines().nth(1) {
        Some(path) if path.starts_with("/devtools/browser/") => Ok(path.to_owned()),
        other => Err(format!(
            "{}: second line {other:?} is not /devtools/browser/<id>",
            file.display()
        )),
    }
}

/// The last 20 lines of `path`.
fn tail(path: &Path) -> String {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| format!("({}: {e})", path.display()));
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(20)..].join("\n")
}
