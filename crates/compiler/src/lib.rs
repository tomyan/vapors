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
mod sections;
mod source;
mod template;

use std::path::Path;

pub use codegen::generate;
pub use error::CompileError;
pub use naming::component_name;
use source::Source;

/// The attribute generated code tags each element with, to its
/// component's style scope; reserved in templates. It must equal
/// `vapors_dom::SCOPE_ATTRIBUTE`, which generated code names (checked
/// in vapors-compiler-examples).
pub const SCOPE_ATTRIBUTE: &str = "data-wisp";

/// Parses the text of a `.wisp` file. `path` names the file in
/// refusals and gives the component its name.
///
/// # Errors
/// A [`CompileError`] naming the first thing the parser did not
/// recognise.
pub fn parse_str(path: &str, text: &str) -> Result<ast::Component, CompileError> {
    let source = Source::new(path, text);
    let name = component_name(Path::new(path))?;
    let sections = sections::split(&source)?;
    let template = template::parse(&source, sections.template_start, sections.template_end)?;
    Ok(ast::Component {
        name,
        path: path.to_string(),
        scope: Some(path.to_string()),
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

/// Reads, parses and generates: the Rust for a `.wisp` file.
///
/// # Errors
/// A [`CompileError`] for an unreadable file (positioned at 1:1), or
/// from parsing or code generation.
pub fn compile_file(path: &Path) -> Result<String, CompileError> {
    compile_str(&path.display().to_string(), &read_source(path)?)
}

/// Reads and parses a `.wisp` file.
///
/// # Errors
/// A [`CompileError`] for an unreadable file (positioned at 1:1) or
/// for the first thing the parser did not recognise.
pub fn parse_file(path: &Path) -> Result<ast::Component, CompileError> {
    parse_str(&path.display().to_string(), &read_source(path)?)
}

/// [`compile_file`] with the component's scope named by its path under
/// `root`, so it is the same on every machine.
///
/// # Errors
/// As [`compile_file`]; and a file not under `root`, naming both.
pub fn compile_file_under(path: &Path, root: &Path) -> Result<String, CompileError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        build::path_error(
            path,
            format!(
                "is not under {}, which names its style's scope",
                root.display()
            ),
        )
    })?;
    let mut component = parse_file(path)?;
    component.scope = Some(
        relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    );
    generate(&component)
}

/// [`compile_file`] for a component whose `<style>` the app adds as a
/// global sheet: nothing is scoped.
///
/// # Errors
/// As [`compile_file`].
pub fn compile_file_global(path: &Path) -> Result<String, CompileError> {
    let mut component = parse_file(path)?;
    component.scope = None;
    generate(&component)
}

fn read_source(path: &Path) -> Result<String, CompileError> {
    std::fs::read_to_string(path)
        .map_err(|err| build::path_error(path, format!("cannot read the file: {err}")))
}
