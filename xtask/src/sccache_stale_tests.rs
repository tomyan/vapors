//! A running sccache server whose TMPDIR was a run's, now gone, is
//! refused; anything else is left to run.

use std::path::Path;

use super::*;

fn environ(vars: &[&str]) -> Vec<u8> {
    vars.iter().flat_map(|v| v.bytes().chain([0])).collect()
}

const GONE: &dyn Fn(&Path) -> bool = &|_| false;
const THERE: &dyn Fn(&Path) -> bool = &|_| true;

#[test]
fn a_server_on_our_port_whose_run_tmpdir_is_gone_is_refused_naming_the_fix() {
    // Given
    let env = environ(&["SCCACHE_START_SERVER=1", "TMPDIR=/tmp/xtask-run-4120322"]);

    // When
    let said = stale(&env, "4226", GONE).expect("refused");

    // Then
    assert!(
        said.contains("/tmp/xtask-run-4120322") && said.contains("fix: sccache --stop-server"),
        "{said}"
    );
}

#[test]
fn a_server_whose_tmpdir_is_there_or_not_a_runs_or_unset_is_left_alone() {
    let run = environ(&["SCCACHE_START_SERVER=1", "TMPDIR=/tmp/xtask-run-1"]);
    let other = environ(&["SCCACHE_START_SERVER=1", "TMPDIR=/tmp/elsewhere"]);
    let unset = environ(&["SCCACHE_START_SERVER=1"]);
    assert_eq!(stale(&run, "4226", THERE), None);
    assert_eq!(stale(&other, "4226", GONE), None);
    assert_eq!(stale(&unset, "4226", GONE), None);
}

#[test]
fn a_server_on_another_port_or_a_client_is_not_ours_to_judge() {
    let other_port = environ(&[
        "SCCACHE_START_SERVER=1",
        "SCCACHE_SERVER_PORT=4391",
        "TMPDIR=/tmp/xtask-run-1",
    ]);
    let client = environ(&["TMPDIR=/tmp/xtask-run-1"]);
    assert_eq!(stale(&other_port, "4226", GONE), None);
    assert_eq!(stale(&client, "4226", GONE), None);
}
