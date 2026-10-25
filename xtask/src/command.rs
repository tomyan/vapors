//! Running one step and capturing what it produced.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::steps::Step;

/// What happened when a step ran.
pub struct Outcome {
    /// True when the process exited with status zero.
    pub passed: bool,
    /// Combined stdout and stderr, in the order each stream was read.
    pub output: String,
    /// Wall-clock time the step took.
    pub duration: Duration,
}

/// Run a step from its directory and capture its output; past its
/// deadline its process group is killed and the step fails saying so.
///
/// A step that cannot be started (program missing) is reported as a
/// failure whose output names the program, never as a panic.
pub fn run(step: &Step) -> Outcome {
    let started = Instant::now();
    let (passed, output) = match spawn(step) {
        Ok(child) => finish(child, step.deadline),
        Err(error) => (false, error),
    };
    Outcome {
        passed,
        output,
        duration: started.elapsed(),
    }
}

/// The step's process, the leader of its own process group so that a
/// deadline kills what it started too, with the compilation cache set
/// (`sccache`), except for sccache's own step.
fn spawn(step: &Step) -> Result<Child, String> {
    // Outside the gate, under nice 10 with the job cap (`priority`).
    let mut command = crate::priority::command(&step.program)?;
    // Every step gets the cache, builds or not, and so every step is
    // refused without sccache: one rule, rather than a list of which
    // steps compile that could fall behind. sccache's own step is the
    // exception, since it is the check.
    if step.program != "sccache" {
        crate::sccache::apply(&mut command)?;
    }
    command
        .args(&step.args)
        .envs(step.env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    if let Some(dir) = &step.dir {
        command.current_dir(dir);
    }
    command
        .spawn()
        .map_err(|error| format!("could not start `{}`: {error}", step.program))
}

fn finish(mut child: Child, deadline: Option<Duration>) -> (bool, String) {
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let status = wait(&mut child, deadline);
    let mut output = joined(stdout);
    output.push_str(&joined(stderr));
    match status {
        Ok(passed) => (passed, output),
        Err(why) => (false, format!("{output}\n{why}")),
    }
}

/// Whether it exited zero; an error when it passed `deadline` and was
/// killed, or could not be waited on.
fn wait(child: &mut Child, deadline: Option<Duration>) -> Result<bool, String> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| format!("waiting: {e}"))? {
            return Ok(status.success());
        }
        if deadline.is_some_and(|limit| started.elapsed() > limit) {
            let limit = deadline.unwrap_or_default().as_secs();
            return Err(match kill_group(child) {
                Ok(()) => format!("killed: still running after {limit} s, its deadline"),
                Err(why) => format!(
                    "still running after {limit} s, its deadline, and its group could not be killed: {why}"
                ),
            });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// SIGKILL to the step's process group, then reap the leader, with
/// `kill -s KILL -- -<pid>`, procps's documented form for a group. With
/// `kill -KILL -<pid>` the group survived on the runner: vapors run
/// 40006425576, where the gate's deadline test took 21 s, and likewise vapors
/// #30. The kill's own failure is returned to be reported.
pub(crate) fn kill_group(child: &mut Child) -> Result<(), String> {
    let group = format!("-{}", child.id());
    let killed = Command::new("kill")
        .args(["-s", "KILL", "--", &group])
        .output();
    let _ = child.kill();
    let _ = child.wait();
    match killed {
        Ok(out) if out.status.success() => Ok(()),
        // ESRCH: every process of the group had exited by the kill.
        Ok(out) if String::from_utf8_lossy(&out.stderr).contains("No such process") => Ok(()),
        Ok(out) => Err(format!(
            "kill -s KILL -- {group}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("kill: {e}")),
    }
}

fn drain(stream: Option<impl Read + Send + 'static>) -> Option<JoinHandle<String>> {
    stream.map(|mut stream| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stream.read_to_end(&mut bytes);
            String::from_utf8_lossy(&bytes).into_owned()
        })
    })
}

/// How long the output may stay open after the step's process ended:
/// a process it left behind (outside its group) can hold the pipe.
const DRAIN_GRACE: Duration = Duration::from_secs(10);

/// The stream's text, or a note when it stayed open past the grace.
fn joined(handle: Option<JoinHandle<String>>) -> String {
    let Some(handle) = handle else {
        return String::new();
    };
    let ended = Instant::now();
    while !handle.is_finished() && ended.elapsed() < DRAIN_GRACE {
        std::thread::sleep(Duration::from_millis(50));
    }
    if !handle.is_finished() {
        return format!(
            "(output still open {} s after the step ended; a process it started holds it)\n",
            DRAIN_GRACE.as_secs()
        );
    }
    handle.join().unwrap_or_default()
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
