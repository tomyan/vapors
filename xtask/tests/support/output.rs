//! A child's output under a time limit, its pipes read as it runs.

use std::io::Read;
use std::process::{Child, Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// `command`'s output; past `limit`, the child is killed and the test
/// fails naming the limit and what it printed on both streams. The child
/// stays in the test's process group, so nextest's kill of a test
/// reaches it. Declared limit: a step the child started runs in a group
/// of its own (`command.rs`) and is not killed with it; with the
/// stand-ins, no step outlives its start.
pub fn output_within(mut command: Command, limit: Duration) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("xtask starts");
    let readers = [read(child.stdout.take()), read(child.stderr.take())];
    let finished = wait_within(&mut child, limit);
    let [stdout, stderr] = readers.map(|reader| reader.join().expect("reader"));
    let status = finished.unwrap_or_else(|| {
        panic!(
            "still running after {limit:?}, killed; stdout: {}; stderr: {}",
            String::from_utf8_lossy(&stdout).trim(),
            String::from_utf8_lossy(&stderr).trim()
        )
    });
    Output {
        status,
        stdout,
        stderr,
    }
}

fn read(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
    let mut pipe = pipe.expect("piped");
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        pipe.read_to_end(&mut bytes).expect("readable pipe");
        bytes
    })
}

/// Its exit status, or `None` after killing it at `limit`.
fn wait_within(child: &mut Child, limit: Duration) -> Option<std::process::ExitStatus> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("waitable") {
            return Some(status);
        }
        if started.elapsed() > limit {
            child.kill().expect("kill");
            child.wait().expect("killed child");
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
