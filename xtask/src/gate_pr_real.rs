//! `gate --pr`'s real effects (`gate_pr::Effects`): gh, git, the gate as
//! a child process, and the environment's secrets; and `run`, the
//! command, over them.

use std::io::{BufRead, BufReader};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, ExitCode, Stdio};

use crate::gate_pr::{Effects, Ended, Pr, Ran, run_with};

/// The command: gate PR `number` (or this branch's) and post its tail;
/// the gate's exit code once posted, else a failure.
pub fn run(number: Option<u64>) -> ExitCode {
    match run_with(&Real, number) {
        Ok(ended) => {
            println!("gate --pr: posted ({})", ended.words());
            if ended == Ended::Code(0) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(why) => {
            println!("FAIL gate --pr: {why}");
            ExitCode::FAILURE
        }
    }
}

struct Real;

impl Effects for Real {
    fn find(&self, number: Option<u64>) -> Result<Pr, String> {
        crate::gate_pr_gh::find(number)
    }

    fn local_head(&self) -> Result<String, String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .map_err(|e| format!("git: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "git rev-parse HEAD: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn gate(&self) -> Result<Ran, String> {
        let exe = std::env::current_exe().map_err(|e| format!("this xtask's path: {e}"))?;
        let mut child = Command::new(exe)
            .arg("gate")
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("could not start the gate: {e}"))?;
        let output = match passed_through(&mut child) {
            Ok(output) => output,
            Err(why) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(why);
            }
        };
        let status = child
            .wait()
            .map_err(|e| format!("waiting for the gate: {e}"))?;
        let ended = match (status.code(), status.signal()) {
            (Some(code), _) => Ended::Code(code),
            (None, Some(signal)) => Ended::Signal(signal),
            (None, None) => return Err("the gate ended with neither a code nor a signal".into()),
        };
        Ok(Ran { ended, output })
    }

    fn post(&self, number: u64, body: &str) -> Result<(), String> {
        crate::gate_pr_gh::post(number, body)
    }

    fn secrets(&self) -> Vec<(String, String)> {
        std::env::vars()
            .filter(|(name, _)| secret_name(name))
            .collect()
    }
}

/// The child's output, printed as it comes and kept.
fn passed_through(child: &mut Child) -> Result<String, String> {
    let stdout = child
        .stdout
        .take()
        .ok_or("the gate's output could not be read")?;
    let mut output = String::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|e| format!("reading the gate's output: {e}"))?;
        println!("{line}");
        output.push_str(&line);
        output.push('\n');
    }
    Ok(output)
}

/// Whether an environment variable's name says it holds a secret.
#[must_use]
pub fn secret_name(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    ["_TOKEN", "_KEY", "_SECRET", "_PASSWORD"]
        .iter()
        .any(|end| name.ends_with(end))
        || name == "GITHUB_TOKEN"
        || name == "GH_TOKEN"
}
