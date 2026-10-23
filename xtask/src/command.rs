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
        Err(error) => (
            false,
            format!("could not start `{}`: {error}", step.program),
        ),
    };
    Outcome {
        passed,
        output,
        duration: started.elapsed(),
    }
}

/// The step's process, the leader of its own process group so that a
/// deadline kills what it started too.
fn spawn(step: &Step) -> std::io::Result<Child> {
    let mut command = Command::new(&step.program);
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
    command.spawn()
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
            kill_group(child);
            let limit = deadline.unwrap_or_default().as_secs();
            return Err(format!(
                "killed: still running after {limit} s, its deadline"
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// SIGKILL to the step's process group, then reap the leader.
fn kill_group(child: &mut Child) {
    let _ = Command::new("kill")
        .args(["-KILL", &format!("-{}", child.id())])
        .status();
    let _ = child.kill();
    let _ = child.wait();
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
