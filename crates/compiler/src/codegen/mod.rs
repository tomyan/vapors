//! Code generation: a parsed component to one readable Rust file.
//!
//! The file holds the `<style>` text as a constant, a props struct
//! with defaults, the component struct holding its signals and memos,
//! and `mount_before`, which runs the `<script>` statements and then
//! builds the template through the `Dom` trait: each `{expr}` hole
//! and expression attribute gets its own effect, each `{if}` and
//! `{for}` an anchor and a call into `vapors_dom::blocks`.

mod allowances;
mod blocks;
mod component_tag;
mod element;
mod emit;
mod format;
mod script;
mod script_syntax;
mod shell;
mod syn_map;
mod text;
mod unwrap;

use crate::CompileError;
use crate::ast::Component;
use crate::source::Source;
use emit::{Emitter, Place};

/// Generates the Rust for a parsed component.
///
/// # Errors
/// A [`CompileError`] for a script or expression that does not parse,
/// a signal or prop without a type, a binding that cannot be wired,
/// or a feature that is deferred (snippets, component children).
pub fn generate(component: &Component) -> Result<String, CompileError> {
    let source = Source::new(&component.path, &component.text);
    let info = script::analyse(&source, component.script.as_ref())?;
    let mut emitter = Emitter::new(&source, &info);
    emitter.indent(3);
    emitter.emit_nodes(&component.template.children, &Place::roots())?;
    let (body, allowances) = emitter.finish();
    let raw = shell::assemble(component, &info, &body, allowances);
    let formatted = format::format(&source, &raw)?;
    // A plain comment, not `//!`: the file is `include!`d into a module,
    // where inner attributes are not allowed.
    Ok(format!(
        "// Generated from `{}` by vapors-compiler; edit the .wisp, not this file.\n\n{formatted}",
        component.path
    ))
}
