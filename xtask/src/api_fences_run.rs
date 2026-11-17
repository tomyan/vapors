//! `cargo xtask api-fences <name>`: makes builder `<name>`'s clone, starts
//! the harness server, and proves the sandbox's fences (`api_fences.rs`)
//! and what must work inside (`api_fences_works.rs`), as `api-build`
//! will before every session. The builder's directories are removed
//! afterwards, and a removal that fails fails the run.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use crate::api_fences::{CANARY, Escapes, Ran, SENTINEL, probes, prove, ran};
use crate::api_fences_works::{Expected, Works, checked, host_namespaces};
use crate::api_paths::{Builder, KEY_FILE};
use crate::api_sandbox::{BIN, Host, RELAY_PORT, arguments};

/// Proves builder `name`'s fences.
pub fn run(name: &str) -> ExitCode {
    let proven = crate::api_fds::refuse_inherited()
        .and_then(|()| crate::api_host::home())
        .and_then(|home| Builder::new(name, &home))
        .and_then(|builder| made_and_proven(&builder));
    match proven {
        Ok(lines) => {
            for line in &lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(why) => {
            println!("FAIL api-fences: {why}");
            ExitCode::FAILURE
        }
    }
}

/// The builder made, its fences proven, and its directories removed;
/// a builder already there is refused and left as it is.
fn made_and_proven(builder: &Builder) -> Result<Vec<String>, String> {
    let (main, objects) = crate::api_clone::main_repository()?;
    crate::api_clone::make(builder, (&main, &objects), &branch(builder))?;
    let proven = crate::api_host::host(objects).and_then(|host| proven(builder, &host));
    let removed = removed(builder);
    let lines = proven?;
    removed.map(|()| lines)
}

fn proven(builder: &Builder, host: &Host) -> Result<Vec<String>, String> {
    crate::api_server::serve(&builder.socket())?;
    let inside = |script: &str| inside(builder, host, script);
    let key_file = builder.home.join(KEY_FILE);
    let escapes = Escapes {
        hidden: builder.root().join("escape-probe"),
        shared_cache: builder.home.join(".cache/sccache/api-build-probe"),
        read_only: &host.objects,
        key_file: &key_file,
        clone_dir: &builder.clone_dir(),
    };
    let mut lines = prove(&probes(&escapes), &inside)?;
    lines.extend(checked(&works(builder, host)?, &inside)?);
    Ok(lines)
}

/// What must work inside for `builder` on `host`.
fn works(builder: &Builder, host: &Host) -> Result<Vec<Works>, String> {
    let ping = crate::api_server::PING;
    let refs = format!(
        "refs/heads/{}\n{}",
        branch(builder),
        crate::api_clone::ORIGIN_MAIN
    );
    let names = crate::api_fences_works::NAMESPACES.join(" ");
    let home = crate::api_sandbox::home_entries(builder, host).join("\n");
    let work = |name, script: String, expected| Works {
        name,
        script,
        expected,
    };
    Ok(vec![
        work(
            "the way in reaches the harness server",
            format!("curl -q -sS --max-time 15 http://127.0.0.1:{RELAY_PORT}{ping}"),
            Expected::Is("pong".into()),
        ),
        work(
            "cargo resolves the workspace offline",
            "cargo metadata --offline --format-version 1 > /dev/null && echo resolved".into(),
            Expected::Is("resolved".into()),
        ),
        work(
            "namespaces of its own",
            format!("for n in {names}; do readlink /proc/self/ns/$n; done"),
            Expected::NoneOf(host_namespaces()?),
        ),
        work(
            "a session of its own",
            "s=$(cut -d' ' -f6 /proc/self/stat); test \"$s\" != 0 && test -e /proc/$s && echo leads".into(),
            Expected::Is("leads".into()),
        ),
        work("only the bound entries in the home directory", "ls -A \"$HOME\" | sort".into(), Expected::Is(home)),
        work(
            "only the slice's branch and origin/main in the clone",
            "git for-each-ref --format='%(refname)' | sort".into(),
            Expected::Is(refs),
        ),
        work(
            "no descriptor inherited",
            "ls /proc/$$/fd | sort -n | paste -sd' '".into(),
            Expected::Is("0 1 2".into()),
        ),
    ])
}

/// The branch a builder's session works on.
pub fn branch(builder: &Builder) -> String {
    format!("api/{}", builder.name)
}

/// `script` run by bash inside builder's sandbox, under the relay, saying
/// [`SENTINEL`] first.
fn inside(builder: &Builder, host: &Host, script: &str) -> Result<Ran, String> {
    let script = format!("echo '{SENTINEL}'; {script}");
    let mut command = sandboxed(builder, host, &["bash", "-c", &script]);
    command.env(CANARY.0, CANARY.1);
    ran(command)
}

/// `command` to run inside builder's sandbox, under the relay, its stdin
/// /dev/null.
pub fn sandboxed(builder: &Builder, host: &Host, command: &[&str]) -> Command {
    let mut bwrap = Command::new("bwrap");
    bwrap
        .args(arguments(builder, host, RELAY_PORT))
        .stdin(Stdio::null());
    bwrap.arg(Path::new(BIN).join("xtask"));
    let port = RELAY_PORT.to_string();
    bwrap.args(["api-relay-inner", "--port", &port, "--socket"]);
    bwrap.arg(builder.socket());
    bwrap.arg("--").args(command);
    bwrap
}

/// The builder's directories removed.
///
/// # Errors
///
/// Each that could not be, so the run fails and the next run under the
/// name is not refused unexplained.
fn removed(builder: &Builder) -> Result<(), String> {
    let failed: Vec<String> = [builder.root(), builder.target()]
        .iter()
        .filter_map(|directory| {
            std::fs::remove_dir_all(directory)
                .err()
                .map(|e| format!("could not remove {}: {e}", directory.display()))
        })
        .collect();
    if failed.is_empty() {
        return Ok(());
    }
    Err(failed.join("; "))
}
