//! Loose text inside a body: literal runs with `{expr}` holes, and the
//! rule that collapses whitespace-only runs.

use super::Parser;
use super::entities::decode;
use super::holes::Kind;
use crate::CompileError;
use crate::ast::{Expr, Node, Part, Text};

impl Parser<'_> {
    /// Reads text up to the next tag or block; see `collapse_whitespace`
    /// for the one whitespace rule.
    pub fn parse_loose_text(&mut self) -> Result<Option<Node>, CompileError> {
        let offset = self.pos;
        let mut parts = Vec::new();
        let mut literal = String::new();
        let mut literal_start = self.pos;
        loop {
            match self.peek() {
                None | Some('<') => break,
                Some('{') => {
                    let hole = self.peek_hole()?;
                    if hole.kind != Kind::Expr {
                        break;
                    }
                    self.push_literal(&mut parts, &mut literal, literal_start)?;
                    parts.push(Part::Expr(Expr {
                        code: hole.body,
                        offset: hole.body_offset,
                    }));
                    self.bump(hole.len);
                }
                Some('}') => {
                    return Err(self.error(self.pos, "stray `}` in text: it has no matching `{`"));
                }
                Some(c) => {
                    if literal.is_empty() {
                        literal_start = self.pos;
                    }
                    literal.push(c);
                    self.bump(c.len_utf8());
                }
            }
        }
        self.push_literal(&mut parts, &mut literal, literal_start)?;
        Ok(text_node(parts, offset))
    }

    /// Ends a literal run that began at `start`, decoding its
    /// character references.
    fn push_literal(
        &self,
        parts: &mut Vec<Part>,
        literal: &mut String,
        start: usize,
    ) -> Result<(), CompileError> {
        if literal.is_empty() {
            return Ok(());
        }
        let part = match whitespace_run(literal) {
            Run::Kept => Some(Part::Literal(
                decode(literal).map_err(|(at, message)| self.error(start + at, message))?,
            )),
            Run::Dropped => None,
            Run::Space => Some(Part::Literal(" ".into())),
        };
        literal.clear();
        parts.extend(part);
        Ok(())
    }
}

fn text_node(parts: Vec<Part>, offset: usize) -> Option<Node> {
    (!parts.is_empty()).then_some(Node::Text(Text { parts, offset }))
}

/// What the whitespace rule does with a literal run.
enum Run {
    /// It has something other than whitespace: kept as written.
    Kept,
    /// Whitespace-only with a newline: formatting, dropped.
    Dropped,
    /// Whitespace-only on one line: one space.
    Space,
}

/// A whitespace-only run always sits between two non-text things (an
/// element, a tag boundary or a hole), since adjacent text is one run:
/// it is dropped when it contains a newline and is one space otherwise.
/// The check is on the raw text, so `&nbsp;` is never collapsed.
fn whitespace_run(raw: &str) -> Run {
    if !raw.chars().all(|c| c.is_ascii_whitespace()) {
        Run::Kept
    } else if raw.contains('\n') {
        Run::Dropped
    } else {
        Run::Space
    }
}
