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
mod prop_suggestion;
mod props_index;
mod script;
mod script_syntax;
mod script_uses;
mod shell;
mod snippets;
mod syn_map;
mod tag_path;
mod tag_props;
mod text;
mod unwrap;

use crate::CompileError;
use crate::ast::Component;
use crate::source::Source;
use emit::{Emitter, Place};
pub use props_index::PropsIndex;

/// The template emitted with node counters of `width` digits.
fn emit_template<'a>(
    component: &Component,
    source: &'a Source,
    info: &'a script::ScriptInfo,
    (width, index): (usize, Option<&'a PropsIndex>),
) -> Result<Emitter<'a>, Vec<CompileError>> {
    let mut emitter = Emitter::new(source, info).with_width(width);
    if let Some(index) = index {
        emitter = emitter.with_index(index);
    }
    let mut emitter = emitter.with_scope(component.style.is_some() && component.scope.is_some());
    emitter.indent(3);
    // a refusal stops the emission; the props refused before it stand
    if let Err(err) = emitter.emit_nodes(&component.template.children, &Place::roots()) {
        let mut errors = std::mem::take(&mut emitter.tags.errors);
        errors.push(err);
        return Err(errors);
    }
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
    generated(component, None)
        .map(|(code, _)| code)
        .map_err(|mut errors| errors.remove(0))
}

/// [`generate`], checking each component tag against the props `index`
/// holds for the component it names.
///
/// # Errors
/// As [`generate`]; and a tag passing a prop its component does not
/// declare, the first such.
pub fn generate_with(component: &Component, index: &PropsIndex) -> Result<String, CompileError> {
    generate_checked(component, index)
        .map(|generated| generated.code)
        .map_err(|mut errors| errors.remove(0))
}

/// The props check's findings for one component.
struct Checked {
    errors: Vec<CompileError>,
    unchecked: Vec<String>,
}

/// What [`generate_checked`] made of a component.
#[derive(Debug)]
pub struct Generated {
    /// The Rust.
    pub code: String,
    /// `file:line:col: ..` for each component tag the index could not
    /// resolve, so its props were not checked.
    pub unchecked: Vec<String>,
}

/// [`generate_with`], with every prop refusal and the unchecked tags.
///
/// # Errors
/// Every prop a tag passes that its component does not declare; or the
/// one refusal [`generate`] gives.
pub fn generate_checked(
    component: &Component,
    index: &PropsIndex,
) -> Result<Generated, Vec<CompileError>> {
    let (code, tags) = generated(component, Some(index))?;
    if !tags.errors.is_empty() {
        return Err(tags.errors);
    }
    Ok(Generated {
        code,
        unchecked: tags.unchecked,
    })
}

fn generated(
    component: &Component,
    index: Option<&PropsIndex>,
) -> Result<(String, Checked), Vec<CompileError>> {
    let source = Source::new(&component.path, &component.text);
    let info = script::analyse(&source, component.script.as_ref()).map_err(|err| vec![err])?;
    let mut emitter = emit_template(component, &source, &info, (1, index))?;
    let width = digits(emitter.names_made().saturating_sub(1));
    if width > 1 {
        emitter = emit_template(component, &source, &info, (width, index))?;
    }
    names::check_generated(&source, &emitter.declared(), emitter.generated())
        .map_err(|err| vec![err])?;
    let tags = std::mem::take(&mut emitter.tags);
    let tags = Checked {
        errors: tags.errors,
        unchecked: tags.unchecked,
    };
    let body = emitter.finish();
    let raw = shell::assemble(component, &info, &body);
    let formatted = format::format(&source, &raw).map_err(|err| vec![err])?;
    // A plain comment, not `//!`: the file is `include!`d into a module,
    // where inner attributes are not allowed.
    Ok((
        format!(
            "// Generated from `{}` by vapors-compiler; edit the .wisp, not this file.\n\n{formatted}",
            component.path
        ),
        tags,
    ))
}
