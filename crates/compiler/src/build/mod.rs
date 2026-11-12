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
pub use files::{Styles, compile_dir_into, compile_dir_into_styled};

/// What [`compile_dir_into`] produced.
#[derive(Debug)]
pub struct Compiled {
    /// Every directory scanned and `.wisp` file compiled, sorted:
    /// what cargo must watch.
    pub inputs: Vec<PathBuf>,
    /// The `wisp.rs` file to `include!`.
    pub modules: PathBuf,
    /// `file:line:col: ..` for each component tag that names no
    /// component in the directory, so its props were not checked.
    pub unchecked: Vec<String>,
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
    run(src.as_ref(), Styles::Scoped);
}

/// [`compile_dir`] for components whose `<style>` the app adds as a
/// global sheet: nothing is scoped. The layout corpus uses it, so its
/// CSS means what Chromium reads.
pub fn compile_dir_global(src: impl AsRef<Path>) {
    run(src.as_ref(), Styles::Global);
}

fn run(src: &Path, styles: Styles) {
    let src = resolve_src(src, std::env::var_os("CARGO_MANIFEST_DIR"));
    let report = styled_report(&src, std::env::var_os("OUT_DIR"), styles);
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
    styled_report(src, out_dir, Styles::Scoped)
}

fn styled_report(src: &Path, out_dir: Option<OsString>, styles: Styles) -> BuildScriptReport {
    let Some(out_dir) = out_dir else {
        return failed(src, vec![missing_out_dir()]);
    };
    match compile_dir_into_styled(src, Path::new(&out_dir), styles) {
        Ok(compiled) => BuildScriptReport {
            stdout: rerun_lines(&compiled.inputs)
                .into_iter()
                .chain(
                    compiled
                        .unchecked
                        .iter()
                        .map(|note| format!("cargo::warning={note}")),
                )
                .collect(),
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
