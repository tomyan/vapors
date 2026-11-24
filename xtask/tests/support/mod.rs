//! Running the xtask binary in a scratch directory that cannot reach
//! the workspace or the machine's sccache server: its HOME, its own
//! TMPDIR, an environment of four variables, and a PATH holding only a
//! stand-in `cargo`, a stand-in `sccache` and the system's tools.
//! Without this, a child `xtask ci` or `test` that got past its wait
//! found the workspace above `target/tmp` (where `target` is a plain
//! directory) and ran the real tiers, which ran these tests again: the
//! `quiet_wiring` timeouts and nested leftovers of issue #358.

pub mod gate;
mod output;

pub use output::output_within;

use std::path::{Path, PathBuf};
use std::process::Command;

/// What the stand-in `cargo` does when a child gets past its wait and
/// starts a step: write the environment it was given to
/// `$HOME/cargo.env`, say so and fail, doing no work.
pub const NO_WORK: &str =
    "#!/bin/sh\nenv > \"$HOME/cargo.env\"\necho \"stand-in cargo: $*\"\nexit 1\n";

/// The stand-in `sccache`: it starts no server, and its statistics are
/// the shape `sccache::parse` reads, with a location other than a local
/// disk so no cache size is checked. A child can then never start a
/// real server whose cache would live in the scratch HOME.
/// Each call is appended to `$HOME/sccache.log`.
const SCCACHE: &str = r#"#!/bin/sh
echo "$*" >> "$HOME/sccache.log"
case "$*" in
  --start-server) exit 0 ;;
  *--show-stats*) echo '{"cache_location":"stand-in","stats":{"cache_hits":{"counts":{}},"cache_misses":{"counts":{}},"cache_errors":{"counts":{}},"cache_read_errors":0,"cache_write_errors":0}}' ;;
  *) echo "stand-in sccache: $*"; exit 1 ;;
esac
"#;

/// `dir/bin` holding `cargo` (the script `cargo`) and the stand-in
/// `sccache`.
pub fn bin(dir: &Path, cargo: &str) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin dir");
    script(&bin.join("cargo"), cargo);
    script(&bin.join("sccache"), SCCACHE);
    bin
}

fn script(path: &Path, text: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, text).expect("stand-in");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("mode");
}

/// The xtask binary run in `dir` (given an empty `.config/factory.toml`
/// when it has none) with an empty environment but for four variables: `HOME` (`dir`), `TMPDIR` (`tmp`), `PATH` (`bin`, then the
/// system's directories) and `GIT_CEILING_DIRECTORIES` (so git cannot
/// look above `dir`). A test adds what its case needs with `env`.
pub fn xtask(dir: &Path, bin: &Path, tmp: &Path) -> Command {
    let parent = dir.parent().expect("a scratch dir has a parent");
    // a workspace always has a settings file, which the build
    // directory's cap finds the workspace by; an empty one sets nothing
    let settings = dir.join(".config/factory.toml");
    if !settings.exists() {
        std::fs::create_dir_all(dir.join(".config")).expect("settings dir");
        std::fs::write(&settings, "").expect("settings");
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"));
    child
        .current_dir(dir)
        .env_clear()
        .env("HOME", dir)
        .env("TMPDIR", tmp)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("GIT_CEILING_DIRECTORIES", parent);
    child
}

/// Every entry directly under `dir`, for an assertion that it is empty.
pub fn entries(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("readable").flatten() {
        found.push(entry.path().display().to_string());
    }
    found
}
