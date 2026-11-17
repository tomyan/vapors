//! The `.wisp` component compiler: one file holding a `<script>` of
//! Rust, a `<style>` of CSS and an HTML template, parsed into an AST
//! and generated into readable Rust over the `vapors-dom` builder
//! trait. `docs/components.md` describes the format and what is
//! refused; every refusal is a [`CompileError`] naming
//! `file:line:col`.

pub mod ast;
pub mod build;
mod codegen;
mod error;
mod naming;
mod scene_notes;
mod sections;
mod shape_table;
mod source;
mod style_check;
pub mod styles;
mod template;

use std::path::Path;

use build::Styles;
pub use codegen::{Generated, PropsIndex, generate, generate_checked, generate_with};
pub use error::CompileError;
pub use naming::component_name;
pub use scene_notes::scene_notes;
use source::Source;

/// The attribute generated code tags each element with, to its
/// component's style scope; reserved in templates. It must equal
/// `vapors_dom::SCOPE_ATTRIBUTE`, which generated code names (checked
/// in vapors-compiler-examples).
pub const SCOPE_ATTRIBUTE: &str = "data-wisp";

/// How many `class:` directives one element may have: a tree keeps
/// which are on as bits of a `u64`. It must equal
/// `vapors_dom::MAX_TOGGLES`, which refuses more where the element is
/// created (checked in vapors-compiler-examples).
pub const MAX_CLASS_DIRECTIVES: usize = 64;

/// Parses the text of a `.wisp` file. `path` names the file in
/// refusals and gives the component its name. Its `<style>` is read as
/// a scoped style (see [`parse_file_global`] for the other kind).
///
/// # Errors
/// A [`CompileError`] naming the first thing the parser did not
/// recognise, in the template or the style.
pub fn parse_str(path: &str, text: &str) -> Result<ast::Component, CompileError> {
    parse_text(path, text, Styles::Scoped)
}

fn parse_text(path: &str, text: &str, styles: Styles) -> Result<ast::Component, CompileError> {
    let source = Source::new(path, text);
    let name = component_name(Path::new(path))?;
    let sections = sections::split(&source)?;
    if let Some(style) = &sections.style {
        style_check::check(&source, style, styles)?;
    }
    let template = template::parse(&source, sections.template_start, sections.template_end)?;
    Ok(ast::Component {
        name,
        path: path.to_string(),
        scope: Some(path.to_string()),
        global_sheet: None,
        text: text.to_string(),
        script: sections.script,
        style: sections.style,
        template_offset: sections.template_start,
        template,
    })
}

/// Parses and generates: the Rust for a `.wisp` file's text.
///
/// # Errors
/// A [`CompileError`] from parsing or from code generation.
pub fn compile_str(path: &str, text: &str) -> Result<String, CompileError> {
    generate(&parse_str(path, text)?)
}

/// Reads and parses a `.wisp` file, its scope named by the path as
/// given. Not public: generated code from it would name that path, which
/// may be the build machine's; callers name a root (`parse_file_under`).
///
/// # Errors
/// A [`CompileError`] for an unreadable file (positioned at 1:1) or
/// for the first thing the parser did not recognise.
fn parse_file(path: &Path) -> Result<ast::Component, CompileError> {
    parse_str(&path.display().to_string(), &read_source(path)?)
}

/// Reads, parses and generates: the Rust for a `.wisp` file, its scope
/// named by its path under `root`, so it is the same on every machine.
///
/// # Errors
/// A [`CompileError`] for an unreadable file (positioned at 1:1), from
/// parsing or code generation, or a file not under `root`, naming both.
pub fn compile_file_under(path: &Path, root: &Path) -> Result<String, CompileError> {
    generate(&parse_file_under(path, root)?)
}

/// Reads and parses a `.wisp` file, its scope named by its path under
/// `root`; refusals name the path as given, which an editor opens.
///
/// # Errors
/// A [`CompileError`] for an unreadable file (positioned at 1:1), for
/// the first thing the parser did not recognise, or a file not under
/// `root`, naming both.
pub fn parse_file_under(path: &Path, root: &Path) -> Result<ast::Component, CompileError> {
    let relative = relative_name(path, root)?;
    let mut component = parse_file(path)?;
    component.scope = Some(relative);
    Ok(component)
}

/// [`parse_file_under`] for a component whose `<style>` is a global
/// sheet, which may hold no `:global()`: its sheet named by its path
/// under `root`.
///
/// # Errors
/// As [`parse_file_under`].
pub fn parse_file_global_under(path: &Path, root: &Path) -> Result<ast::Component, CompileError> {
    let relative = relative_name(path, root)?;
    let mut component = parse_file_global(path)?;
    component.global_sheet = Some(relative);
    Ok(component)
}

/// `path` under `root`, `/`-separated: the same on every machine.
fn relative_name(path: &Path, root: &Path) -> Result<String, CompileError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        build::path_error(
            path,
            format!(
                "is not under {}, which names its style's sheet",
                root.display()
            ),
        )
    })?;
    let parts = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy());
    Ok(parts.collect::<Vec<_>>().join("/"))
}

/// [`compile_file_under`] for a component whose `<style>` the app adds
/// as a global sheet: nothing is scoped.
///
/// # Errors
/// As [`compile_file_under`].
pub fn compile_file_global(path: &Path) -> Result<String, CompileError> {
    // its sheet named by its file under its own directory, the same on
    // every machine, as `compile_dir_global` names it under the root
    let root = path.parent().unwrap_or_else(|| Path::new(""));
    generate(&parse_file_global_under(path, root)?)
}

/// [`parse_file`] for a component whose `<style>` is a global sheet,
/// which may hold no `:global()`. Not public, as [`parse_file`].
///
/// # Errors
/// As [`parse_file`].
fn parse_file_global(path: &Path) -> Result<ast::Component, CompileError> {
    let text = read_source(path)?;
    let mut component = parse_text(&path.display().to_string(), &text, Styles::Global)?;
    component.scope = None;
    component.global_sheet = Some(path.display().to_string());
    Ok(component)
}

fn read_source(path: &Path) -> Result<String, CompileError> {
    std::fs::read_to_string(path)
        .map_err(|err| build::path_error(path, format!("cannot read the file: {err}")))
}
