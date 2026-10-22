//! The template parser: a cursor over the file's text between the
//! template's start and end, and the loop that reads one body of
//! nodes up to its closer.

mod attributes;
mod children;
mod control_flow;
mod elements;
mod entities;
mod holes;
mod snippets;
mod text;
mod vocabulary;

use crate::CompileError;
use crate::ast::{Document, Node};
use crate::source::Source;

/// Parses the template between `start` and `end`.
pub(crate) fn parse(source: &Source, start: usize, end: usize) -> Result<Document, CompileError> {
    let mut parser = Parser {
        source,
        pos: start,
        end,
    };
    let mut children = Vec::new();
    loop {
        parser.skip_whitespace();
        if parser.at_end() {
            return Ok(Document { children });
        }
        children.push(parser.parse_top_level()?);
    }
}

pub(crate) struct Parser<'a> {
    source: &'a Source,
    pos: usize,
    end: usize,
}

impl Parser<'_> {
    pub fn rest(&self) -> &str {
        &self.source.text[self.pos..self.end]
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.end
    }

    pub fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.rest().starts_with(prefix)
    }

    pub fn bump(&mut self, bytes: usize) {
        self.pos += bytes;
    }

    pub fn skip_whitespace(&mut self) {
        let trimmed = self.rest().trim_start().len();
        self.pos = self.end - trimmed;
    }

    /// Reads while `keep` holds and returns what was read.
    pub fn read_while(&mut self, keep: impl Fn(char) -> bool) -> &str {
        let rest = self.rest();
        let len = rest.find(|c: char| !keep(c)).unwrap_or(rest.len());
        let start = self.pos;
        self.pos += len;
        &self.source.text[start..start + len]
    }

    pub fn error(&self, offset: usize, message: impl Into<String>) -> CompileError {
        self.source.error(offset, message)
    }

    pub fn describe(&self, offset: usize) -> String {
        self.source.describe(offset)
    }

    /// A top-level node: an element or a block. Text goes inside an
    /// element.
    fn parse_top_level(&mut self) -> Result<Node, CompileError> {
        match self.peek() {
            Some('<') if self.starts_with("</") => Err(self.unopened_close_tag()),
            Some('<') => self.parse_element(),
            Some('{') => self.parse_top_level_block(),
            Some(other) => Err(self.error(
                self.pos,
                format!("unexpected `{other}` at the top level: text goes inside an element"),
            )),
            None => unreachable!("checked at_end before parsing a node"),
        }
    }

    fn parse_top_level_block(&mut self) -> Result<Node, CompileError> {
        let token = self.peek_hole()?;
        match &token.kind {
            holes::Kind::If | holes::Kind::For | holes::Kind::Snippet | holes::Kind::Render => {
                self.parse_block(&token)
            }
            holes::Kind::Expr => Err(self.error(
                token.offset,
                "unexpected `{` at the top level: text and expressions go inside an element",
            )),
            _ => Err(self.error(
                token.offset,
                format!("`{{{}}}` has no open block to close", token.body),
            )),
        }
    }

    pub fn unopened_close_tag(&self) -> CompileError {
        let name = self.rest()[2..]
            .split(|c: char| c == '>' || c.is_whitespace())
            .next()
            .unwrap_or_default();
        self.error(self.pos, format!("`</{name}>` has no open tag to close"))
    }
}
