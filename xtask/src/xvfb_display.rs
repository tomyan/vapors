//! Choosing a free X display number for Xvfb. A display is taken while
//! its lock file (`/tmp/.X<n>-lock`) names a running process, or while
//! its socket (`/tmp/.X11-unix/X<n>`) exists without a lock. A lock
//! left by a process that is no longer running, and owned by this
//! user, is removed with its socket so killed Xvfb runs do not use up
//! the range.

use std::fs;
use std::ops::RangeInclusive;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Where to look and how to tell a running process, injected so tests
/// run over a fake `/tmp`.
pub struct DisplaySearch<'a> {
    /// The directory holding `.X<n>-lock` and `.X11-unix/`; `/tmp`.
    pub root: &'a Path,
    /// The user whose stale locks may be removed.
    pub uid: u32,
    /// Whether a process id is running.
    pub is_running: &'a dyn Fn(u32) -> bool,
}

/// A display number that is free, and a line for each stale lock that
/// was removed to free one.
#[derive(Debug, PartialEq, Eq)]
pub struct Claimed {
    /// The display number, such as 99 for `:99`.
    pub number: u32,
    /// One line per lock removed.
    pub removed: Vec<String>,
}

/// What one display number's lock says.
enum Lock {
    Absent,
    Live,
    Stale { pid: u32 },
    OtherUsers,
}

impl DisplaySearch<'_> {
    /// The first free display in `range`, removing stale locks on the
    /// way; `None` when every one is taken.
    ///
    /// # Errors
    ///
    /// A lock file that cannot be read, has no process id, or cannot be
    /// removed; the message names the file.
    pub fn claim(&self, range: RangeInclusive<u32>) -> Result<Option<Claimed>, String> {
        let mut removed = Vec::new();
        for number in range {
            let free = match self.lock(number)? {
                Lock::Absent => !self.socket(number).exists(),
                Lock::Live | Lock::OtherUsers => false,
                Lock::Stale { pid } => {
                    removed.push(self.remove_stale(number, pid)?);
                    true
                }
            };
            if free {
                return Ok(Some(Claimed { number, removed }));
            }
        }
        Ok(None)
    }

    fn lock(&self, number: u32) -> Result<Lock, String> {
        let path = self.lock_path(number);
        if !path.exists() {
            return Ok(Lock::Absent);
        }
        let pid = read_pid(&path)?;
        if (self.is_running)(pid) {
            return Ok(Lock::Live);
        }
        let owner = fs::metadata(&path)
            .map_err(|e| unreadable(&path, &e))?
            .uid();
        Ok(if owner == self.uid {
            Lock::Stale { pid }
        } else {
            Lock::OtherUsers
        })
    }

    fn remove_stale(&self, number: u32, pid: u32) -> Result<String, String> {
        let lock = self.lock_path(number);
        fs::remove_file(&lock)
            .map_err(|e| format!("{}: could not remove a stale X lock: {e}", lock.display()))?;
        let socket = self.socket(number);
        if socket.exists() {
            fs::remove_file(&socket).map_err(|e| {
                format!(
                    "{}: could not remove a stale X socket: {e}",
                    socket.display()
                )
            })?;
        }
        Ok(format!(
            "removed {} and its socket: process {pid} is not running",
            lock.display()
        ))
    }

    fn lock_path(&self, number: u32) -> PathBuf {
        self.root.join(format!(".X{number}-lock"))
    }

    fn socket(&self, number: u32) -> PathBuf {
        self.root.join(format!(".X11-unix/X{number}"))
    }
}

/// The process id Xvfb writes into its lock: ten columns and a newline.
fn read_pid(path: &Path) -> Result<u32, String> {
    let text = fs::read_to_string(path).map_err(|e| unreadable(path, &e))?;
    text.trim().parse().map_err(|_| {
        format!(
            "{}: an X lock holding {:?}, not a process id; remove it if no X server uses it",
            path.display(),
            text.trim()
        )
    })
}

fn unreadable(path: &Path, error: &std::io::Error) -> String {
    format!("{}: could not read this X lock: {error}", path.display())
}

/// Whether `pid` is running, from `/proc`.
pub fn process_is_running(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

/// This process's user id, from the owner of `/proc/self`.
///
/// # Errors
///
/// When `/proc/self` cannot be read.
pub fn current_uid() -> Result<u32, String> {
    fs::metadata("/proc/self")
        .map(|m| m.uid())
        .map_err(|e| format!("/proc/self: could not read this process's user id: {e}"))
}

#[cfg(test)]
#[path = "xvfb_display_tests.rs"]
mod tests;
