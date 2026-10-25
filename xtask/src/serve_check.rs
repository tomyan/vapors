//! `[serve] check`: a shell line run in the checkout on the built page,
//! within `[serve] check_seconds`. The build is served only when it
//! exits 0 in time; otherwise its output is shown with the refusal.

use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};

/// Run `check` by `sh -c`, within `deadline`.
///
/// # Errors
///
/// It exited non-zero or passed the deadline; its output and the fix.
pub fn run_check(check: &str, deadline: Duration) -> Result<(), String> {
    let mut command = Command::new("sh");
    command.args(["-c", check]);
    match run_with_deadline(command, deadline)? {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!(
            "`{check}` refused the built page:\n{output}fix: what it names, then serve again"
        )),
        Finished::TimedOut { output } => Err(format!(
            "`{check}` did not finish within {}s:\n{output}fix: find why it stalls, or raise [serve] check_seconds from a measurement",
            deadline.as_secs()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(5);

    #[test]
    fn a_check_that_exits_zero_passes() {
        assert_eq!(run_check("true", SECOND), Ok(()));
    }

    #[test]
    fn a_check_that_fails_is_refused_with_its_output() {
        // When
        let refused = run_check("echo no dev keys here; exit 3", SECOND).expect_err("exit 3");

        // Then
        assert!(refused.contains("refused the built page"), "{refused}");
        assert!(refused.contains("no dev keys here"), "{refused}");
    }

    #[test]
    fn a_check_past_its_deadline_is_refused_naming_it() {
        // When
        let refused = run_check("sleep 5", Duration::from_millis(200)).expect_err("too slow");

        // Then
        assert!(refused.contains("did not finish within"), "{refused}");
    }
}
