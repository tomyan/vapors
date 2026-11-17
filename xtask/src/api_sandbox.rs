//! The sandbox an API builder's session runs in (`docs/plans/api-builder.md`):
//! bwrap's arguments. No network but loopback, no process outside it,
//! the home directory an empty tmpfs; writable only the builder's clone,
//! target directory and cargo home, and the sandboxes' own sccache
//! directory, never the host's;
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

/// The most the sandbox's /tmp may hold: a tmpfs is memory, and without a
/// size it may take half the host's. Measured 2026-11-17: the ci steps
/// inside used at most 20 KB of /tmp (sampled every 5 s); 1 GiB leaves room
/// for a test that writes large scratch files and bounds one that loops.
const TMP_BYTES: &str = "1073741824";

/// The most the home tmpfs may hold: the mountpoints of the binds and
/// the CLI's own config, a few MB; 256 MiB bounds what a session can
/// write there.
const HOME_BYTES: &str = "268435456";

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
    /// The GPU render nodes (`/dev/dri/renderD*`), bound for the GPU
    /// tier; the card nodes, which can drive displays, are not.
    pub render_nodes: Vec<PathBuf>,
}

/// bwrap's arguments for `builder` on `host`, the relay on `port`,
/// before the command it runs.
pub fn arguments(builder: &Builder, host: &Host, port: u16) -> Vec<String> {
    let mut args = isolation();
    args.extend(system(&host.render_nodes));
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
        "--unshare-cgroup",
        // its own user namespace, in which it may make no other: nested
        // user namespaces widen the kernel surface a session can reach
        "--unshare-user",
        "--disable-userns",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
    ])
}

fn system(render_nodes: &[PathBuf]) -> Vec<String> {
    let mut args = words(&["--ro-bind", "/usr", "/usr", "--ro-bind", "/etc", "/etc"]);
    for link in ["bin", "lib", "lib64", "sbin"] {
        args.extend(words(&[
            "--symlink",
            &format!("usr/{link}"),
            &format!("/{link}"),
        ]));
    }
    args.extend(words(&[
        "--proc", "/proc", "--dev", "/dev", "--size", TMP_BYTES, "--tmpfs", "/tmp",
    ]));
    for node in render_nodes {
        args.extend(bind("--dev-bind", node, node));
    }
    // Mesa finds the render node's device through sysfs
    args.extend(words(&["--ro-bind", "/sys", "/sys"]));
    args
}

/// The home directory hidden, then what is bound back into it.
fn home(builder: &Builder, host: &Host) -> Vec<String> {
    let mut args = words(&["--size", HOME_BYTES, "--tmpfs"]);
    args.push(text(&builder.home));
    for read_only in read_only(builder, host) {
        args.extend(bind("--ro-bind", &read_only, &read_only));
    }
    for writable in writable(builder) {
        args.extend(bind("--bind", &writable, &writable));
    }
    let registry = builder.cargo_home().join("registry");
    args.extend(bind(
        "--ro-bind",
        &builder.home.join(".cargo/registry"),
        &registry,
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

/// What is bound read-only at its own path: the toolchain, the factory's
/// pinned downloads (ci's tools step checks them, `downloads.rs`), the
/// main repository's objects (the clone's alternates) and the socket's
/// directory.
fn read_only(builder: &Builder, host: &Host) -> [PathBuf; 5] {
    let at = |relative: &str| builder.home.join(relative);
    [
        at(".rustup"),
        at(".cargo/bin"),
        at(".local/share/factory"),
        host.objects.clone(),
        builder.socket_dir(),
    ]
}

/// What the session writes: its cargo home, clone and target, and the
/// sandboxes' own sccache directory. The host's shared sccache directory
/// is never bound: a cache entry written inside would be linked into a
/// host build (review of #467, H1).
fn writable(builder: &Builder) -> [PathBuf; 4] {
    [
        builder.cargo_home(),
        builder.clone_dir(),
        builder.target(),
        builder.sandbox_sccache(),
    ]
}

/// The entries of the home directory inside: the first part of every
/// path bound under it, sorted.
pub fn home_entries(builder: &Builder, host: &Host) -> Vec<String> {
    let mut bound: Vec<PathBuf> = read_only(builder, host)
        .into_iter()
        .chain(writable(builder))
        .collect();
    bound.push(builder.cargo_home().join("registry"));
    let mut entries: Vec<String> = bound
        .iter()
        .filter_map(|path| path.strip_prefix(&builder.home).ok()?.components().next())
        .map(|first| first.as_os_str().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    entries.dedup();
    entries
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
        ("SCCACHE_DIR", text(&builder.sandbox_sccache())),
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
