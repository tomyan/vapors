//! `{if}` and `{for}` blocks.

use super::Parser;
use super::children::{Closer, Ended};
use super::holes::{Hole, Kind};
use crate::CompileError;
use crate::ast::{Expr, ForBlock, IfBlock, Node};

impl Parser<'_> {
    /// Parses the block whose opening hole is `hole` (not yet consumed).
    pub fn parse_block(&mut self, hole: &Hole) -> Result<Node, CompileError> {
        match hole.kind {
            Kind::If => self.parse_if(hole),
            Kind::For => self.parse_for(hole),
            Kind::Snippet => self.parse_snippet(hole),
            Kind::Render => self.parse_render(hole),
            _ => unreachable!("only opening holes reach parse_block"),
        }
    }

    fn parse_if(&mut self, hole: &Hole) -> Result<Node, CompileError> {
        let condition = after_keyword(hole, "if");
        if condition.code.is_empty() {
            return Err(self.error(
                hole.offset,
                "`{if}` needs a condition: `{if count.get() > 0}`",
            ));
        }
        self.bump(hole.len);
        let closer = Closer::If {
            opened: hole.offset,
        };
        let (then, ended) = self.parse_children(&closer)?;
        let otherwise = match ended {
            Ended::Closed => None,
            Ended::Else => Some(self.parse_children(&closer)?.0),
        };
        Ok(Node::If(IfBlock {
            condition,
            then,
            otherwise,
            offset: hole.offset,
        }))
    }

    fn parse_for(&mut self, hole: &Hole) -> Result<Node, CompileError> {
        let clause = after_keyword(hole, "for");
        let Some(in_at) = clause.code.find(" in ") else {
            return Err(self.error(
                hole.offset,
                "`{for}` needs `pattern in expression`, like `{for (i, x) in items.get() key=x.id}`",
            ));
        };
        let pattern = clause.code[..in_at].trim().to_string();
        let (iterable, key) = split_key(&clause, in_at + 4);
        self.bump(hole.len);
        let closer = Closer::For {
            opened: hole.offset,
        };
        let (children, _) = self.parse_children(&closer)?;
        Ok(Node::For(ForBlock {
            pattern,
            iterable,
            key,
            children,
            offset: hole.offset,
        }))
    }
}

/// The hole's body after its keyword, trimmed, with its offset.
pub(crate) fn after_keyword(hole: &Hole, keyword: &str) -> Expr {
    let rest = &hole.body[keyword.len()..];
    let leading = rest.len() - rest.trim_start().len();
    Expr {
        code: rest.trim().to_string(),
        offset: hole.body_offset + keyword.len() + leading,
    }
}

/// Splits `iterable key=expr` on the last ` key=`, so a name that
/// contains `key` does not confuse it.
fn split_key(clause: &Expr, from: usize) -> (Expr, Option<Expr>) {
    let rest = &clause.code[from..];
    let Some(key_at) = rest.rfind(" key=") else {
        return (sub_expr(clause, from, clause.code.len()), None);
    };
    let iterable = sub_expr(clause, from, from + key_at);
    let key = sub_expr(clause, from + key_at + 5, clause.code.len());
    (iterable, Some(key))
}

fn sub_expr(clause: &Expr, start: usize, end: usize) -> Expr {
    let raw = &clause.code[start..end];
    let leading = raw.len() - raw.trim_start().len();
    Expr {
        code: raw.trim().to_string(),
        offset: clause.offset + start + leading,
    }
}
