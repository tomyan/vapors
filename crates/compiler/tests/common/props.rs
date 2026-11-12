//! Helpers for the props-check tests: a directory holding `plain.wisp`
//! (one prop, `title`) and a parent page.

#![allow(dead_code)] // each test binary uses a subset

use std::path::PathBuf;

use crate::build::{dirs, write};
use vapors_compiler::CompileError;
use vapors_compiler::build::{Compiled, compile_dir_into};

pub const PLAIN: &str =
    "<script>\n#[prop]\nlet title: String = String::new();\n</script>\n\n<h2>{title}</h2>";

/// The script line every page starts with, importing `Plain`.
pub const USE_PLAIN: &str = "<script>\nuse crate::plain::{Plain, PlainProps};\n</script>\n\n";

/// `src` and `out` for a test, with `plain.wisp` in `src`.
pub fn with_plain(name: &str) -> (PathBuf, PathBuf) {
    let (src, out) = dirs(name);
    write(&src, "plain.wisp", PLAIN);
    (src, out)
}

/// The refusals of a directory holding `plain.wisp` and a page whose
/// template is `parent`, after the `use` of `Plain`.
pub fn refusals(name: &str, parent: &str) -> Vec<String> {
    let (src, out) = with_plain(name);
    write(&src, "page.wisp", &format!("{USE_PLAIN}{parent}"));
    shown(compile_dir_into(&src, &out))
}

/// The refusals, as printed; none when it compiled.
pub fn shown(compiled: Result<Compiled, Vec<CompileError>>) -> Vec<String> {
    match compiled {
        Ok(_) => Vec::new(),
        Err(errors) => errors.iter().map(ToString::to_string).collect(),
    }
}
