//! A child process's output, or a refusal when it outlives its
//! deadline: a hung child would hold a quiet window until its hold
//! expired (review of #428, L5).

use std::io::Read;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

/// How often the child is polled.
const POLL: Duration = Duration::from_millis(50);

/// `command`'s output, killing it at `deadline`. Its pipes are read on
/// threads of their own, so a child that writes more than a pipe holds
/// is not blocked until the deadline.
///
/// # Errors
/// It cannot start, its output cannot be read, or it ran past
/// `deadline`, naming the command and the deadline.
pub fn output_within(command: &mut Command, deadline: Duration) -> Result<Output, String> {
    let shown = format!("{command:?}");
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("starting {shown}: {e}"))?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let status = wait_within(&mut child, deadline, &shown)?;
    let joined = |reader: std::thread::JoinHandle<std::io::Result<Vec<u8>>>| {
        reader
            .join()
            .map_err(|_| format!("reading {shown}'s output panicked"))?
            .map_err(|e| format!("reading {shown}'s output: {e}"))
    };
    Ok(Output {
        status,
        stdout: joined(stdout)?,
        stderr: joined(stderr)?,
    })
}

fn drain(
    pipe: Option<impl Read + Send + 'static>,
) -> std::thread::JoinHandle<std::io::Result<Vec<u8>>> {
    std::thread::spawn(move || {
        let mut out = Vec::new();
        if let Some(mut pipe) = pipe {
            pipe.read_to_end(&mut out)?;
        }
        Ok(out)
    })
}

fn wait_within(child: &mut Child, deadline: Duration, shown: &str) -> Result<ExitStatus, String> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("waiting on {shown}: {e}"))?
        {
            return Ok(status);
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{shown} ran past its deadline of {deadline:?} and was killed; run it alone to see \
                 where it stops"
            ));
        }
        std::thread::sleep(POLL);
    }
}

#[cfg(test)]
#[path = "child_deadline_tests.rs"]
mod tests;
