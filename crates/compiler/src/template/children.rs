//! Reading a body of nodes up to the closer that ends it: a close tag
//! or a block terminator, with `{else}` ending an `{if}` body.

use super::Parser;
use super::holes::Kind;
use super::vocabulary::is_void;
use crate::CompileError;
use crate::ast::Node;

/// What ends the body being read, and where it opened.
pub(crate) enum Closer {
    Element { tag: String, opened: usize },
    If { opened: usize },
    For { opened: usize },
    Snippet { name: String, opened: usize },
}

impl Closer {
    fn expected(&self) -> String {
        match self {
            Closer::Element { tag, .. } => format!("</{tag}>"),
            Closer::If { .. } => "{/if}".into(),
            Closer::For { .. } => "{/for}".into(),
            Closer::Snippet { .. } => "{/snippet}".into(),
        }
    }

    fn opened(&self) -> usize {
        match self {
            Closer::Element { opened, .. }
            | Closer::If { opened }
            | Closer::For { opened }
            | Closer::Snippet { opened, .. } => *opened,
        }
    }

    fn describe(&self) -> String {
        match self {
            Closer::Element { tag, .. } => format!("<{tag}>"),
            Closer::If { .. } => "{if}".into(),
            Closer::For { .. } => "{for}".into(),
            Closer::Snippet { name, .. } => format!("{{snippet {name}}}"),
        }
    }
}

/// How a body ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Ended {
    Closed,
    Else,
}

enum Step {
    Node(Node),
    Ended(Ended),
}

impl Parser<'_> {
    /// Reads nodes until the closer, consuming it.
    pub fn parse_children(&mut self, closer: &Closer) -> Result<(Vec<Node>, Ended), CompileError> {
        let mut children = Vec::new();
        loop {
            if self.at_end() {
                return Err(self.unclosed(closer));
            }
            if self.starts_with("</") {
                self.parse_close_tag(closer)?;
                return Ok((children, Ended::Closed));
            }
            if self.starts_with("<") {
                children.push(self.parse_element()?);
            } else if self.starts_with("{") && self.peek_hole()?.kind != Kind::Expr {
                match self.parse_block_or_terminator(closer)? {
                    Step::Node(node) => children.push(node),
                    Step::Ended(ended) => return Ok((children, ended)),
                }
            } else if let Some(text) = self.parse_loose_text()? {
                children.push(text);
            }
        }
    }

    fn unclosed(&self, closer: &Closer) -> CompileError {
        self.error(
            closer.opened(),
            format!(
                "`{}` is never closed: add `{}` after its content",
                closer.describe(),
                closer.expected()
            ),
        )
    }

    fn parse_close_tag(&mut self, closer: &Closer) -> Result<(), CompileError> {
        let offset = self.pos;
        self.bump(2);
        let name = self.read_while(is_tag_char).to_string();
        self.skip_whitespace();
        if !self.starts_with(">") {
            return Err(self.error(offset, format!("expected `>` to finish `</{name}`")));
        }
        self.bump(1);
        match closer {
            Closer::Element { tag, .. } if *tag == name => Ok(()),
            _ if is_void(&name) => Err(self.error(
                offset,
                format!("`<{name}>` is a void element and has no close tag: remove `</{name}>`"),
            )),
            _ => Err(self.error(
                offset,
                format!(
                    "expected `{}` to close `{}` opened at {}, found `</{name}>`",
                    closer.expected(),
                    closer.describe(),
                    self.describe(closer.opened())
                ),
            )),
        }
    }

    /// A block node, or how the body ended when the hole was its terminator.
    fn parse_block_or_terminator(&mut self, closer: &Closer) -> Result<Step, CompileError> {
        let token = self.peek_hole()?;
        match &token.kind {
            Kind::If | Kind::For | Kind::Snippet | Kind::Render => {
                self.parse_block(&token).map(Step::Node)
            }
            Kind::Else => {
                if !matches!(closer, Closer::If { .. }) {
                    return Err(self.error(token.offset, "`{else}` outside an `{if}` block"));
                }
                self.bump(token.len);
                Ok(Step::Ended(Ended::Else))
            }
            Kind::ElseIf => Err(self.error(
                token.offset,
                "there is no `{else if}`: nest an `{if}` inside the `{else}` branch",
            )),
            Kind::Close(keyword) => self
                .close_block(closer, &token, keyword)
                .map(|()| Step::Ended(Ended::Closed)),
            Kind::Slot => Err(self.error(token.offset, super::vocabulary::SLOT_REMOVED)),
            Kind::Expr => unreachable!("expression holes are read as text"),
        }
    }

    fn close_block(
        &mut self,
        closer: &Closer,
        token: &super::holes::Hole,
        keyword: &str,
    ) -> Result<(), CompileError> {
        let matches = match closer {
            Closer::If { .. } => keyword == "if",
            Closer::For { .. } => keyword == "for",
            Closer::Snippet { .. } => keyword == "snippet",
            Closer::Element { .. } => false,
        };
        if !matches {
            return Err(self.error(
                token.offset,
                format!(
                    "expected `{}` to close `{}` opened at {}, found `{{/{keyword}}}`",
                    closer.expected(),
                    closer.describe(),
                    self.describe(closer.opened())
                ),
            ));
        }
        self.bump(token.len);
        Ok(())
    }
}

pub(crate) fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '_' | ':')
}
