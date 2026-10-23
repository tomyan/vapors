//! One external command the factory runs, and how to reproduce it.

use std::fmt::Write;
use std::time::Duration;

/// One external command with the advice to print when it fails.
pub struct Step {
    /// Short name shown in the report, such as `fmt`.
    pub name: String,
    /// The program and its arguments, run from the workspace root.
    pub program: String,
    /// Arguments for `program`.
    pub args: Vec<String>,
    /// What to do when this step fails.
    pub fix: String,
    /// Set in the step's environment, in this order.
    pub env: Vec<(String, String)>,
    /// Where it runs, relative to the workspace root; there when `None`.
    pub dir: Option<String>,
    /// Killed, with its process group, and failed when it runs longer.
    pub deadline: Option<Duration>,
}

impl Step {
    pub(crate) fn new(name: &str, program: &str, args: &[&str], fix: &str) -> Self {
        Self {
            name: name.to_owned(),
            program: program.to_owned(),
            args: args.iter().map(|&a| a.to_owned()).collect(),
            fix: fix.to_owned(),
            env: Vec::new(),
            dir: None,
            deadline: None,
        }
    }

    /// The command line a reader can paste to reproduce this step; an
    /// argument a shell would split or expand is single-quoted.
    pub fn reproduce(&self) -> String {
        let mut line = String::new();
        if let Some(dir) = &self.dir {
            let _ = write!(line, "cd {} && ", shell_word(dir));
        }
        for (name, value) in &self.env {
            let _ = write!(line, "{name}={} ", shell_word(value));
        }
        line.push_str(&self.program);
        for arg in &self.args {
            line.push(' ');
            line.push_str(&shell_word(arg));
        }
        line
    }
}

fn shell_word(arg: &str) -> String {
    let plain = arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./=,:+@%".contains(c));
    if plain && !arg.is_empty() {
        return arg.to_owned();
    }
    format!("'{}'", arg.replace('\'', "'\\''"))
}
