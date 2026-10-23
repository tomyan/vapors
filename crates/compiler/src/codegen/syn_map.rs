//! Parsing template expressions and the script with syn, mapping a
//! syntax error's span back to `file:line:col`.

use proc_macro2::LineColumn;

use crate::CompileError;
use crate::ast::Expr;
use crate::source::Source;

/// The file offset of a (line, column) inside `text`, which starts at
/// `base` in the file. `line` is 1-based; `column` counts characters.
pub(crate) fn offset_in(text: &str, base: usize, at: LineColumn) -> usize {
    let line_start = text
        .split_inclusive('\n')
        .take(at.line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    let line = &text[line_start.min(text.len())..];
    let column_bytes: usize = line.chars().take(at.column).map(char::len_utf8).sum();
    base + line_start + column_bytes
}

/// Parses a template expression, positioning any error in the file.
pub(crate) fn parse_expr(source: &Source, expr: &Expr) -> Result<syn::Expr, CompileError> {
    syn::parse_str::<syn::Expr>(&expr.code).map_err(|err| {
        source.error(
            error_offset(expr, &err),
            format!("in `{{{}}}`: {err}", expr.code),
        )
    })
}

/// Where a syntax error points: its span, or the end of the expression
/// when the input ran out (syn gives that error no position of its own).
fn error_offset(expr: &Expr, err: &syn::Error) -> usize {
    if err.to_string().starts_with("unexpected end of input") {
        return expr.offset + expr.code.len();
    }
    offset_in(&expr.code, expr.offset, err.span().start())
}

/// Renders syn tokens as source; prettyplease lays the file out later.
pub(crate) fn render(tokens: &impl quote::ToTokens) -> String {
    tokens.to_token_stream().to_string()
}
