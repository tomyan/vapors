use std::path::Path;

use super::{in_force, no_core_dumps, scope};

#[test]
fn the_scope_carries_the_memory_and_task_limits_without_swap() {
    let args: Vec<String> = scope(Path::new("/x/xtask"))
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    for property in ["MemoryMax=32G", "MemorySwapMax=0", "TasksMax=4096"] {
        assert!(
            args.windows(2).any(|w| w == ["-p", property]),
            "{property} missing: {args:?}"
        );
    }
    assert_eq!(
        &args[args.len() - 4..],
        ["--", "/x/xtask", "api-scoped", "--"]
    );
}

/// A cgroup directory holding `memory.max` and `pids.max` as given.
fn cgroup(memory: Option<&str>, pids: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a scratch directory");
    let dir = root.path().join("user.slice/run-1.scope");
    std::fs::create_dir_all(&dir).expect("made");
    if let Some(memory) = memory {
        std::fs::write(dir.join("memory.max"), format!("{memory}\n")).expect("written");
    }
    std::fs::write(dir.join("pids.max"), format!("{pids}\n")).expect("written");
    root
}

#[test]
fn a_scope_with_its_limits_in_force_passes() {
    let root = cgroup(Some("34359738368"), "4096");
    assert_eq!(
        in_force("0::/user.slice/run-1.scope\n", root.path()),
        Ok(())
    );
}

#[test]
fn a_scope_whose_controller_is_not_delegated_is_refused() {
    let root = cgroup(None, "4096");
    let refused = in_force("0::/user.slice/run-1.scope\n", root.path()).expect_err("refused");
    assert!(refused.contains("memory.max is not there"), "{refused}");
}

#[test]
fn a_scope_whose_limit_is_another_is_refused() {
    let root = cgroup(Some("max"), "4096");
    let refused = in_force("0::/user.slice/run-1.scope\n", root.path()).expect_err("refused");
    assert!(refused.contains("is max, not 34359738368"), "{refused}");
}

#[test]
fn this_process_makes_no_core_dump_once_asked() {
    assert_eq!(no_core_dumps(), Ok(()));
    let limits = std::fs::read_to_string("/proc/self/limits").expect("limits");
    let core = limits
        .lines()
        .find(|line| line.starts_with("Max core file size"))
        .expect("a core line");
    assert!(
        core.split_whitespace().skip(4).take(2).eq(["0", "0"]),
        "{core}"
    );
}
