//! A run started inside another run's step (a test running `xtask`,
//! as `quiet_wiring` does) makes its TMPDIR inside the outer run's. When
//! that inner run is killed, it cannot remove its directory, and before
//! issue #358's fix the outer run then failed its leftover check naming
//! `xtask-run-<pid>`. The outer run now removes a dead inner run's
//! directory, as the start-of-run sweep removes a dead run's at the top.

#[allow(
    dead_code,
    reason = "each test binary compiles `support` and uses part of it"
)]
mod support;

/// The outer run's `cargo` starts an inner `xtask test`, waits until the
/// inner run has made its TMPDIR and started its own step, then kills
/// both with SIGKILL, which no handler could catch. Its poll is bounded
/// at 80 of 50 ms, 4 s, under [`LIMIT`], so a stuck inner run is named
/// by the script rather than by the outer kill.
const OUTER: &str = r#"#!/bin/sh
if [ -n "$NESTED" ]; then echo $$ > "$HOME/inner.pid"; exec sleep 30; fi
NESTED=1 "$XTASK_BIN" test > "$HOME/inner.out" 2>&1 &
inner=$!
n=0
until [ -f "$HOME/inner.pid" ] && [ -d "$TMPDIR/xtask-run-$inner" ]; do
  n=$((n+1))
  if [ $n -gt 80 ]; then echo "the inner run never started its step"; kill -9 $inner; exit 2; fi
  sleep 0.05
done
kill -9 $inner "$(cat "$HOME/inner.pid")"
wait $inner
exit 0
"#;

/// The outer run's limit. Measured 2026-11-14 over 10 runs at 1-minute
/// load 24 to 28: at most 637 ms from start to exit. 5 s is about eight
/// times that, and under nextest's 60 s, so a hang fails here, named.
const LIMIT: std::time::Duration = std::time::Duration::from_secs(5);

#[test]
fn a_killed_inner_runs_directory_is_removed_by_the_outer_run_saying_so() {
    // Given
    let scratch = tempfile::Builder::new()
        .prefix("xtask-killed-inner-")
        .tempdir()
        .expect("scratch");
    let dir = scratch.path();
    let tmp = dir.join("tmp");
    std::fs::create_dir(&tmp).expect("tmp");
    let bin = support::bin(dir, OUTER);

    // When
    let mut outer = support::xtask(dir, &bin, &tmp);
    outer
        .arg("test")
        .env("XTASK_BIN", env!("CARGO_BIN_EXE_xtask"));
    let started = std::time::Instant::now();
    let output = support::output_within(outer, LIMIT);
    println!("the outer run took {:?}", started.elapsed());

    // Then
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{text}");
    assert!(
        text.contains("tmp: removed") && text.contains("which is not going"),
        "{text}"
    );
    assert_eq!(support::entries(&tmp), Vec::<String>::new());
}
