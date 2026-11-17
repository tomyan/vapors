//! The sandbox an API builder's session runs in (`docs/plans/api-builder.md`):
//! bwrap's arguments. No network but loopback, no process outside it,
//! the home directory an empty tmpfs; writable only the builder's clone,
//! target directory and cargo home, and the shared sccache directory;
//! read-only the system, the toolchain, cargo's registry, the CLI, the
//! trusted xtask binary, the main repository's objects (the clone's
//! alternates) and the socket's directory. The fences are proven on every
//! run (`api_fences.rs`).

use std::path::{Path, PathBuf};

use crate::api_paths::Builder;

/// Where the trusted binaries are inside: the xtask that relays and the
/// CLI, at fixed paths on the sandbox's `PATH`.
pub const BIN: &str = "/opt/api-build";

/// The relay's port inside: the sandbox's network is its own, so any
/// port is free there.
pub const RELAY_PORT: u16 = 8080;

/// The key the session sees: not a key. The harness adds the real one
/// outside (`docs/plans/api-builder.md`).
pub const DUMMY_KEY: &str = "sk-ant-api-build-dummy";

/// The paths outside that the sandbox reads, found once per run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Host {
    /// The xtask binary this run is, built from main.
    pub xtask: PathBuf,
    /// The CLI's binary, its symlink resolved.
    pub claude: PathBuf,
    /// The main repository's object directory, the clone's alternates.
    pub objects: PathBuf,
    /// Whether `/dev/dri` exists, to bind for the GPU tier.
    pub dri: bool,
}

/// bwrap's arguments for `builder` on `host`, the relay on `port`,
/// before the command it runs.
pub fn arguments(builder: &Builder, host: &Host, port: u16) -> Vec<String> {
    let mut args = isolation();
    args.extend(system(host.dri));
    args.extend(home(builder, host));
    args.extend(environment(builder, port));
    args
}

fn isolation() -> Vec<String> {
    words(&[
        "--unshare-net",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
    ])
}

fn system(dri: bool) -> Vec<String> {
    let mut args = words(&["--ro-bind", "/usr", "/usr", "--ro-bind", "/etc", "/etc"]);
    for link in ["bin", "lib", "lib64", "sbin"] {
        args.extend(words(&[
            "--symlink",
            &format!("usr/{link}"),
            &format!("/{link}"),
        ]));
    }
    args.extend(words(&[
        "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp",
    ]));
    if dri {
        args.extend(words(&["--dev-bind", "/dev/dri", "/dev/dri"]));
    }
    args
}

/// The home directory hidden, then what is bound back into it.
fn home(builder: &Builder, host: &Host) -> Vec<String> {
    let at = |relative: &str| builder.home.join(relative);
    let mut args = vec!["--tmpfs".into(), text(&builder.home)];
    for read_only in [at(".rustup"), at(".cargo/bin"), host.objects.clone()] {
        args.extend(bind("--ro-bind", &read_only, &read_only));
    }
    let registry = builder.cargo_home().join("registry");
    args.extend(bind("--bind", &builder.cargo_home(), &builder.cargo_home()));
    args.extend(bind("--ro-bind", &at(".cargo/registry"), &registry));
    for writable in [builder.clone_dir(), builder.target(), at(".cache/sccache")] {
        args.extend(bind("--bind", &writable, &writable));
    }
    args.extend(bind(
        "--ro-bind",
        &builder.socket_dir(),
        &builder.socket_dir(),
    ));
    args.extend(bind(
        "--ro-bind",
        &host.xtask,
        &Path::new(BIN).join("xtask"),
    ));
    args.extend(bind(
        "--ro-bind",
        &host.claude,
        &Path::new(BIN).join("claude"),
    ));
    args.extend(["--chdir".into(), text(&builder.clone_dir())]);
    args
}

fn environment(builder: &Builder, port: u16) -> Vec<String> {
    let path = format!(
        "{BIN}:{}:/usr/bin:/bin",
        text(&builder.home.join(".cargo/bin"))
    );
    let variables = [
        ("HOME", text(&builder.home)),
        ("PATH", path),
        ("LANG", "C.UTF-8".into()),
        ("CARGO_HOME", text(&builder.cargo_home())),
        ("RUSTUP_HOME", text(&builder.home.join(".rustup"))),
        ("CARGO_NET_OFFLINE", "true".into()),
        ("CARGO_INCREMENTAL", "0".into()),
        ("SCCACHE_DIR", text(&builder.home.join(".cache/sccache"))),
        ("ANTHROPIC_BASE_URL", format!("http://127.0.0.1:{port}")),
        ("ANTHROPIC_API_KEY", DUMMY_KEY.into()),
        // ask_lead waits up to 31 minutes (api_mcp_inner.rs); the CLI's
        // MCP tool timeout must outlast it
        ("MCP_TOOL_TIMEOUT", (35 * 60 * 1000).to_string()),
    ];
    let mut args = Vec::new();
    for (name, value) in variables {
        args.extend(["--setenv".into(), name.into(), value]);
    }
    args
}

fn bind(kind: &str, from: &Path, to: &Path) -> Vec<String> {
    vec![kind.into(), text(from), text(to)]
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|&word| word.to_owned()).collect()
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
#[path = "api_sandbox_tests.rs"]
mod tests;
