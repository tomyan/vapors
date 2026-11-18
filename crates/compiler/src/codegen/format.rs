//! The last step: the assembled text must parse as a Rust file (or the
//! compiler has a bug), and prettyplease lays it out.

use std::fmt::Write;

use crate::CompileError;
use crate::source::Source;

pub(crate) fn format(source: &Source, raw: &str) -> Result<String, CompileError> {
    match syn::parse_file(raw) {
        Ok(file) => Ok(comment_markers(&prettyplease::unparse(&file))),
        Err(err) => Err(source.error(
            0,
            format!(
                "vapors-compiler generated Rust that does not parse, which is a bug in the compiler; report it with this file. {err} at generated line {}:\n{}",
                err.span().start().line,
                numbered(raw)
            ),
        )),
    }
}

fn numbered(text: &str) -> String {
    let mut numbered = String::new();
    for (index, line) in text.lines().enumerate() {
        let _ = writeln!(numbered, "{:4} {line}", index + 1);
    }
    numbered
}

/// Turns each marker line (`const _: &str = "from f.wisp:3";`) into the
/// comment prettyplease cannot keep.
fn comment_markers(text: &str) -> String {
    let opening = format!("const _: &str = \"{}", super::script_hoist::MARK);
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&opening) {
            let indent = &line[..line.len() - trimmed.len()];
            let from = rest.trim_end_matches("\";");
            let _ = writeln!(out, "{indent}// {}{from}", super::script_hoist::MARK);
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}
