//! Around the Chromium process: the deadline, its output, the files it
//! reads and writes, and where its binary is.

use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use super::chromium::ChromiumError;
use super::measure::Rgba;

/// One screenshot: 0.65 s here. On the runner, the whole comparison
/// example (both renders and both screenshots) took 6.85 s: run
/// 39989880425, gpu job 120538591130, from `Running
/// target/debug/examples/text_comparison --out target` at 16:51:07.13
/// to `composite written target/text_comparison.png` at 16:51:13.98.
/// 20 s, about three times that whole, is the ceiling for one screenshot
/// before the run is declared hung. It must stay low enough that the comparison
/// test's screenshots time out here, printing Chromium's stderr, before
/// nextest kills the test: run 39937335978's gpu job lost the stderr
/// that way at 60 s. The tests assert it against .config/nextest.toml.
pub(crate) const DEADLINE: Duration = Duration::from_secs(20);

/// Delete an earlier screenshot so a stale image can never be read back.
pub(crate) fn remove_stale(png: &Path) -> Result<(), ChromiumError> {
    match std::fs::remove_file(png) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ChromiumError::Failed(format!(
            "cannot remove the previous {}: {e}",
            png.display()
        ))),
    }
}

pub(crate) fn check_size(image: &Rgba, png: &Path, wanted: [u32; 2]) -> Result<(), ChromiumError> {
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

/// How long Chromium's stderr may stay open once it has exited or been
/// killed, before the run is reported without the rest of it.
const STDERR_GRACE: Duration = Duration::from_secs(2);

/// `stream` read to its end on its own thread, so a chatty Chromium
/// cannot fill the pipe and block; the text arrives on the receiver.
pub(crate) fn drain(stream: impl std::io::Read + Send + 'static) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(read_all(stream));
    });
    receiver
}

/// Chromium's exit status and stderr. Past `deadline` its process group
/// (it and the processes it started, which hold the pipe) is killed and
/// reaped, and the failure carries what it wrote.
pub(crate) fn finish(
    child: &mut Child,
    stderr: &Receiver<String>,
    deadline: Duration,
) -> Result<(ExitStatus, String), ChromiumError> {
    let status = wait_or_kill(child, deadline);
    let text = stderr.recv_timeout(STDERR_GRACE).unwrap_or_else(|_| {
        format!(
            "(stderr still open {}s after the end)",
            STDERR_GRACE.as_secs()
        )
    });
    match status {
        Ok(status) => Ok((status, text)),
        Err(why) => Err(ChromiumError::Failed(format!("{why}; its stderr:\n{text}"))),
    }
}

fn wait_or_kill(child: &mut Child, deadline: Duration) -> Result<ExitStatus, String> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| format!("waiting: {e}"))? {
            return Ok(status);
        }
        if started.elapsed() > deadline {
            let group = match kill_group(child) {
                Ok(()) => "killed with its process group".to_owned(),
                Err(why) => format!("its process group could not be killed ({why})"),
            };
            return Err(format!(
                "did not finish within {}s; {group}",
                deadline.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// SIGKILL to the child's process group (`spawn` makes it the leader),
/// then reap it, so no zombie is left.
///
/// The group survived on the runner once (run 39997553338, job
/// 120564930866: "stderr still open 2s after the end") with
/// `kill -KILL -<pid>`; `--` before a negative id is procps's documented
/// form, and the kill's own failure is now returned to be reported.
fn kill_group(child: &mut Child) -> Result<(), String> {
    let group = format!("-{}", child.id());
    let killed = std::process::Command::new("kill")
        .args(["-s", "KILL", "--", &group])
        .output();
    let _ = child.kill();
    let _ = child.wait();
    match killed {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format!(
            "kill -s KILL -- {group}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("kill: {e}")),
    }
}

pub(crate) fn read_all(mut stream: impl std::io::Read) -> String {
    let mut text = String::new();
    let _ = stream.read_to_string(&mut text);
    text
}

pub(crate) fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
#[path = "chromium_process_tests.rs"]
mod tests;
