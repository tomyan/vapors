//! The sccache server is started by xtask, outside any run's TMPDIR,
//! before a build can start it under one.

use super::*;
use crate::command::Outcome;

fn outcome(passed: bool, output: &str) -> Outcome {
    Outcome {
        passed,
        output: output.to_owned(),
        duration: std::time::Duration::ZERO,
    }
}

#[test]
fn the_start_step_runs_without_a_tmpdir() {
    assert_eq!(start_step().tmpdir, crate::step::TmpDir::Unset);
    assert_eq!(start_step().args, ["--start-server"]);
}

#[test]
fn a_server_started_or_already_running_is_ready_and_any_other_failure_is_refused() {
    assert_eq!(
        judged(&outcome(
            true,
            "sccache: Listening on address 127.0.0.1:4226"
        )),
        Ok(())
    );
    let running = "sccache: error: Server startup failed: Address in use";
    assert_eq!(judged(&outcome(false, running)), Ok(()));
    let refused =
        judged(&outcome(false, "sccache: error: Permission denied")).expect_err("refused");
    assert!(
        refused.contains("Permission denied") && refused.contains("fix: "),
        "{refused}"
    );
}

#[test]
fn a_tmpdir_set_is_given_and_one_unset_is_removed_from_the_inherited_environment() {
    // Given
    let tmpdir_of = |tmpdir: &crate::step::TmpDir| {
        let mut command = std::process::Command::new("true");
        crate::command::set_tmpdir(&mut command, tmpdir);
        let found = command.get_envs().find(|(name, _)| *name == "TMPDIR");
        found.map(|(_, value)| value.map(std::ffi::OsStr::to_os_string))
    };

    // When / Then: Some(None) is "removed"; None is "inherited"
    assert_eq!(tmpdir_of(&crate::step::TmpDir::Unset), Some(None));
    assert_eq!(
        tmpdir_of(&crate::step::TmpDir::Set("/r".into())),
        Some(Some("/r".into()))
    );
    assert_eq!(tmpdir_of(&crate::step::TmpDir::Inherit), None);
}
