//! A virtual X display for the lifetime of one screenshot.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How long Xvfb gets to create its socket before we give up on it.
const STARTUP_DEADLINE: Duration = Duration::from_secs(5);
const FIRST_DISPLAY: u32 = 99;
const LAST_DISPLAY: u32 = 120;

/// A running Xvfb, killed on drop.
pub struct Xvfb {
    child: Child,
    /// The `DISPLAY` value, such as `:99`.
    pub display: String,
}

impl Xvfb {
    /// Start Xvfb on the first free display number, waiting until its
    /// socket exists.
    pub fn start() -> Result<Self, String> {
        let number = free_display().ok_or_else(|| {
            format!("no free X display between :{FIRST_DISPLAY} and :{LAST_DISPLAY}")
        })?;
        let display = format!(":{number}");
        let child = Command::new("Xvfb")
            .args([&display, "-screen", "0", "1280x800x24", "-nolisten", "tcp"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("could not start Xvfb: {e}; install it (apt install xvfb)"))?;
        let mut xvfb = Self { child, display };
        xvfb.wait_for_socket(number)?;
        Ok(xvfb)
    }

    fn wait_for_socket(&mut self, number: u32) -> Result<(), String> {
        let started = Instant::now();
        while !socket_exists(number) {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Err(format!(
                    "Xvfb exited with {status} before creating {}",
                    self.display
                ));
            }
            if started.elapsed() >= STARTUP_DEADLINE {
                return Err(format!(
                    "Xvfb did not create {} within {}s",
                    self.display,
                    STARTUP_DEADLINE.as_secs()
                ));
            }
            thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    }
}

impl Drop for Xvfb {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn socket_exists(number: u32) -> bool {
    Path::new(&format!("/tmp/.X11-unix/X{number}")).exists()
}

fn free_display() -> Option<u32> {
    (FIRST_DISPLAY..=LAST_DISPLAY).find(|n| !socket_exists(*n))
}
