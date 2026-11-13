//! The `<style>` section read with vapors-css at build time: what the
//! runtime would refuse at the first update (a style that does not
//! parse; a `:global()` the scoping cannot honour) fails the build
//! instead, positioned in the `.wisp` file.

use crate::CompileError;
use crate::ast::Section;
use crate::build::Styles;
use crate::source::Source;

/// Parses the style and applies the runtime's `:global()` rule for
/// `styles`: a scoped style may hold `:global()` in a top-level
/// compound only; a global sheet may hold none.
pub(crate) fn check(
    source: &Source,
    section: &Section,
    styles: Styles,
) -> Result<(), CompileError> {
    refuse_other_line_breaks(source, section)?;
    let in_file = |error| positioned(source, section, error);
    let sheet = vapors_css::parse(&section.text).map_err(in_file)?;
    match styles {
        Styles::Scoped => vapors_css::scope::refuse_nested_global(&sheet),
        Styles::Global => vapors_css::scope::refuse_global(&sheet),
    }
    .map_err(in_file)
}

/// vapors-css counts a lone carriage return and a form feed as line
/// breaks (CSS Syntax 3.3); editors and `Source` count `\n` only, so a
/// position after one would name the wrong line. Refused, naming it.
fn refuse_other_line_breaks(source: &Source, section: &Section) -> Result<(), CompileError> {
    let text = section.text.as_bytes();
    let found = text.iter().enumerate().find(|(index, byte)| match byte {
        b'\x0c' => true,
        b'\r' => text.get(index + 1) != Some(&b'\n'),
        _ => false,
    });
    match found {
        Some((index, byte)) => Err(source.error(
            section.offset + index,
            format!(
                "the <style> holds a {} here, which CSS reads as a line break and editors do not; replace it with a newline",
                if *byte == b'\r' { "lone carriage return" } else { "form feed" }
            ),
        )),
        None => Ok(()),
    }
}

/// A refusal positioned in the section's text, moved to the file: the
/// section's first line starts after its open tag, every later line is
/// a file line of its own.
fn positioned(source: &Source, section: &Section, error: vapors_css::Error) -> CompileError {
    let (line, col) = source.position(section.offset);
    let (css_line, css_col) = (error.position.line as usize, error.position.column as usize);
    let (line, col) = if css_line == 1 {
        (line, col + css_col - 1)
    } else {
        (line + css_line - 1, css_col)
    };
    CompileError {
        path: source.path.clone(),
        line,
        col,
        message: error.message,
    }
}
