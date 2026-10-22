//! Loose text inside a body: literal runs with `{expr}` holes, and the
//! JSX newline rule for whitespace-only runs.

use super::Parser;
use super::entities::decode;
use super::holes::Kind;
use crate::CompileError;
use crate::ast::{Expr, Node, Part, Text};

impl Parser<'_> {
    /// Reads text up to the next tag or block. A whitespace-only run
    /// containing a newline is formatting and yields nothing; one on a
    /// single line yields one space.
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
        let decoded = decode(literal).map_err(|(at, message)| self.error(start + at, message))?;
        literal.clear();
        parts.push(Part::Literal(decoded));
        Ok(())
    }
}

fn text_node(parts: Vec<Part>, offset: usize) -> Option<Node> {
    let only_whitespace = match parts.as_slice() {
        [] => return None,
        [Part::Literal(value)] => value.trim().is_empty(),
        _ => false,
    };
    if !only_whitespace {
        return Some(Node::Text(Text { parts, offset }));
    }
    let Part::Literal(value) = &parts[0] else {
        unreachable!("matched above");
    };
    if value.contains('\n') {
        return None;
    }
    Some(Node::Text(Text {
        parts: vec![Part::Literal(" ".into())],
        offset,
    }))
}
