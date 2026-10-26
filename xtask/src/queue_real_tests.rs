use std::path::Path;

use super::*;

#[test]
fn the_gate_is_run_by_the_queue_s_own_binary_in_the_run_worktree() {
    // Given
    let own = std::env::current_exe().expect("own path");

    // When
    let command = gate_command(Path::new("/q/run"), 71).expect("command");

    // Then
    assert_eq!(command.get_program(), own.as_os_str());
    let args: Vec<_> = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(args, ["gate", "--pr", "71"]);
    assert_eq!(command.get_current_dir(), Some(Path::new("/q/run")));
}
