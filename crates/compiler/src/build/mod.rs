//! For `build.rs`: compile every `.wisp` under a directory into
//! `OUT_DIR`, and tell cargo when to run again.
//!
//! ```ignore
//! // build.rs
//! fn main() {
//!     vapors_compiler::build::compile_dir("src");
//! }
//!
//! // src/lib.rs
//! include!(concat!(env!("OUT_DIR"), "/wisp.rs"));
//! ```
//!
//! `wisp.rs` holds one `pub mod` per component, named after its file
//! (`text-input.wisp` is `text_input`, holding `TextInput`), with a
//! nested module per subdirectory.

mod files;
mod modules;
mod tree;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::CompileError;
pub use files::compile_dir_into;

/// What [`compile_dir_into`] produced.
#[derive(Debug)]
pub struct Compiled {
    /// Every directory scanned and `.wisp` file compiled, sorted:
    /// what cargo must watch.
    pub inputs: Vec<PathBuf>,
    /// The `wisp.rs` file to `include!`.
    pub modules: PathBuf,
}

/// What a build script prints, and whether it must fail.
#[derive(Debug)]
pub struct BuildScriptReport {
    /// `cargo::` instructions.
    pub stdout: Vec<String>,
    /// Refusals, one `error: file:line:col: message` per line, as rustc
    /// prints them.
    pub stderr: Vec<String>,
    /// True when the build must stop.
    pub failed: bool,
}

/// Compiles every `.wisp` under `src` into `OUT_DIR` from a build
/// script. On a refusal it prints each as `error: file:line:col: message` and
/// exits with status 1, which fails the build.
pub fn compile_dir(src: impl AsRef<Path>) {
    let src = resolve_src(src.as_ref(), std::env::var_os("CARGO_MANIFEST_DIR"));
    let report = build_script_report(&src, std::env::var_os("OUT_DIR"));
    for line in &report.stdout {
        println!("{line}");
    }
    for line in &report.stderr {
        eprintln!("{line}");
    }
    if report.failed {
        std::process::exit(1);
    }
}

/// `src` joined to the crate's directory when it is relative, so a
/// refusal's path opens from wherever cargo was run.
#[must_use]
pub fn resolve_src(src: &Path, manifest_dir: Option<OsString>) -> PathBuf {
    match manifest_dir {
        Some(dir) if src.is_relative() => Path::new(&dir).join(src),
        _ => src.to_path_buf(),
    }
}

/// The lines [`compile_dir`] prints, given the value of `OUT_DIR`.
#[must_use]
pub fn build_script_report(src: &Path, out_dir: Option<OsString>) -> BuildScriptReport {
    let Some(out_dir) = out_dir else {
        return failed(src, vec![missing_out_dir()]);
    };
    match compile_dir_into(src, Path::new(&out_dir)) {
        Ok(compiled) => BuildScriptReport {
            stdout: rerun_lines(&compiled.inputs),
            stderr: Vec::new(),
            failed: false,
        },
        Err(errors) => failed(
            src,
            errors
                .iter()
                .map(|error| format!("error: {error}"))
                .collect(),
        ),
    }
}

fn failed(src: &Path, stderr: Vec<String>) -> BuildScriptReport {
    BuildScriptReport {
        stdout: rerun_lines(&[src.to_path_buf()]),
        stderr,
        failed: true,
    }
}

fn missing_out_dir() -> String {
    "error: OUT_DIR is not set: call vapors_compiler::build::compile_dir from the crate's build.rs, where cargo sets it".to_string()
}

fn rerun_lines(inputs: &[PathBuf]) -> Vec<String> {
    inputs
        .iter()
        .map(|input| format!("cargo::rerun-if-changed={}", input.display()))
        .collect()
}

/// A refusal about a whole file or directory, positioned at 1:1.
pub(crate) fn path_error(path: &Path, message: impl Into<String>) -> CompileError {
    CompileError {
        path: path.display().to_string(),
        line: 1,
        col: 1,
        message: message.into(),
    }
}
