//! After the unit restarts: something answers 200 on 127.0.0.1:<port>,
//! and that listener is the unit's own main process. `is-active` alone is
//! true as soon as the process is forked, before it binds; with the port
//! taken, the unit restarts in a loop while another server answers.

use std::process::Command;
use std::time::{Duration, Instant};

/// static-web-server answered within 0.2 s of starting on the dev box
/// (2026-10-26, a throwaway run on 127.0.0.1); systemd's restart adds its
/// own. 10 s is far past both.
const LISTEN_DEADLINE: Duration = Duration::from_secs(10);

/// `Ok` once 127.0.0.1:`port` answers `/` with 200 and `unit`'s main
/// process is the one listening.
///
/// # Errors
///
/// Nothing answering by the deadline, or another process on the port,
/// naming it and the fix.
pub fn wait_for_unit(unit: &str, port: u16) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < LISTEN_DEADLINE {
        if answers(port) {
            return owned_by(unit, port);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "nothing answered 200 on 127.0.0.1:{port} within {} s; fix: journalctl --user -u {unit}",
        LISTEN_DEADLINE.as_secs()
    ))
}

fn answers(port: u16) -> bool {
    Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-m",
            "2",
            "-w",
            "%{http_code}",
            &format!("http://127.0.0.1:{port}/"),
        ])
        .output()
        .is_ok_and(|out| out.stdout == b"200")
}

fn owned_by(unit: &str, port: u16) -> Result<(), String> {
    let main = output(
        "systemctl",
        &["--user", "show", "-p", "MainPID", "--value", unit],
    )?;
    owned_by_main(unit, &main, port)
}

/// Whether the listener on `port` is `unit`'s, whose main pid systemd
/// gave as `main` ("0" for a unit not running). Split from [`owned_by`]
/// so its test runs where there is no user bus to ask (an API builder's
/// sandbox): the test gives systemd's answer; `cargo xtask serve` on the
/// host asks systemd itself. That real call, in [`owned_by`], has no test
/// in any tier: a host_ test of it (a transient unit on a free port)
/// passes under `cargo xtask ci` but fails under the gate, whose bwrap
/// puts each step in a user namespace from which `ss -p` cannot name the
/// unit's process (tried 2026-11-17); a mis-spelt argument there would be
/// found by the next `cargo xtask serve`, which refuses loudly.
fn owned_by_main(unit: &str, main: &str, port: u16) -> Result<(), String> {
    let listening = output("ss", &["-ltnpH", &format!("sport = :{port}")])?;
    let owner = listener(&listening);
    if owner.as_ref().is_some_and(|(_, pid)| *pid == main.trim()) {
        return Ok(());
    }
    let held = owner.map_or_else(
        || "a process ss cannot name".to_owned(),
        |(name, pid)| format!("{name} (pid {pid})"),
    );
    Err(format!(
        "127.0.0.1:{port} is held by {held}, not {unit} (main pid {}); fix: change [serve] port in .config/factory.toml, or stop that process",
        main.trim()
    ))
}

/// The process name and pid in `ss -ltnpH`'s `users:(("name",pid=N,fd=M))`.
#[must_use]
pub fn listener(ss: &str) -> Option<(String, String)> {
    let users = &ss[ss.find("users:((\"")? + 9..];
    let name = users.split('"').next()?.to_owned();
    let pid = users
        .split("pid=")
        .nth(1)?
        .split([',', ')'])
        .next()?
        .to_owned();
    Some((name, pid))
}

fn output(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    Err(format!(
        "{program} {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

#[cfg(test)]
#[path = "serve_listen_tests.rs"]
mod tests;
