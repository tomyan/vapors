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
mod for_block;
mod format;
mod names;
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

/// The template emitted with node counters of `width` digits.
fn emit_template<'a>(
    component: &Component,
    source: &'a Source,
    info: &'a script::ScriptInfo,
    width: usize,
) -> Result<Emitter<'a>, CompileError> {
    let mut emitter = Emitter::new(source, info)
        .with_width(width)
        .with_scope(component.style.is_some() && component.scope.is_some());
    emitter.indent(3);
    emitter.emit_nodes(&component.template.children, &Place::roots())?;
    Ok(emitter)
}

/// Decimal digits in `n`: 1 for 0 to 9.
fn digits(n: usize) -> usize {
    n.checked_ilog10().map_or(1, |log| log as usize + 1)
}

/// Generates the Rust for a parsed component. Node names are a stem and
/// a counter of one width for the whole component (`text03`, `text13`
/// in a template of more than ten), found by emitting once to count
/// them; a component of ten names or fewer is emitted once.
///
/// # Errors
/// A [`CompileError`] for a script or expression that does not parse,
/// a signal or prop without a type, a binding that cannot be wired,
/// or a feature that is deferred (snippets, component children).
pub fn generate(component: &Component) -> Result<String, CompileError> {
    let source = Source::new(&component.path, &component.text);
    let info = script::analyse(&source, component.script.as_ref())?;
    let mut emitter = emit_template(component, &source, &info, 1)?;
    let width = digits(emitter.names_made().saturating_sub(1));
    if width > 1 {
        emitter = emit_template(component, &source, &info, width)?;
    }
    names::check_generated(&source, &emitter.declared(), emitter.generated())?;
    let body = emitter.finish();
    let raw = shell::assemble(component, &info, &body);
    let formatted = format::format(&source, &raw)?;
    // A plain comment, not `//!`: the file is `include!`d into a module,
    // where inner attributes are not allowed.
    Ok(format!(
        "// Generated from `{}` by vapors-compiler; edit the .wisp, not this file.\n\n{formatted}",
        component.path
    ))
}
