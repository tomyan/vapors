//! The `.wisp` component compiler: one file holding a `<script>` of
//! Rust, a `<style>` of CSS and an HTML template, parsed into an AST
//! and generated into readable Rust over the `vapors-dom` builder
//! trait. `docs/components.md` describes the format and what is
//! refused; every refusal is a [`CompileError`] naming
//! `file:line:col`.

pub mod ast;
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

/// Reads and parses a `.wisp` file.
///
/// # Errors
/// A [`CompileError`] for an unreadable file (positioned at 1:1) or
/// for the first thing the parser did not recognise.
pub fn parse_file(path: &Path) -> Result<ast::Component, CompileError> {
    let text = std::fs::read_to_string(path).map_err(|err| CompileError {
        path: path.display().to_string(),
        line: 1,
        col: 1,
        message: format!("cannot read the file: {err}"),
    })?;
    parse_str(&path.display().to_string(), &text)
}
