//! `{snippet name(params)}...{/snippet}` and `{render name(args)}`.
//! Parsed as in sumi; code generation for them is deferred.

use super::Parser;
use super::children::Closer;
use super::control_flow::after_keyword;
use super::holes::Hole;
use crate::CompileError;
use crate::ast::{Node, Render, Snippet};

impl Parser<'_> {
    pub fn parse_snippet(&mut self, hole: &Hole) -> Result<Node, CompileError> {
        let (name, params) = self.name_and_parens(hole, "snippet")?;
        self.bump(hole.len);
        let closer = Closer::Snippet {
            name: name.clone(),
            opened: hole.offset,
        };
        let (children, _) = self.parse_children(&closer)?;
        Ok(Node::Snippet(Snippet {
            name,
            params,
            children,
            offset: hole.offset,
        }))
    }

    pub fn parse_render(&mut self, hole: &Hole) -> Result<Node, CompileError> {
        let (name, parens) = self.name_and_parens(hole, "render")?;
        self.bump(hole.len);
        Ok(Node::Render(Render {
            name,
            args: parens[1..parens.len() - 1].trim().to_string(),
            offset: hole.offset,
        }))
    }

    /// `name(...)` after the keyword: the name and the parenthesised part.
    fn name_and_parens(
        &self,
        hole: &Hole,
        keyword: &str,
    ) -> Result<(String, String), CompileError> {
        let rest = after_keyword(hole, keyword);
        let name_len = rest
            .code
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.code.len());
        let name = rest.code[..name_len].to_string();
        let parens = rest.code[name_len..].trim().to_string();
        if name.is_empty() || !parens.starts_with('(') || !parens.ends_with(')') {
            return Err(self.error(
                hole.offset,
                format!(
                    "`{{{keyword}}}` needs a name and a parameter list: `{{{keyword} name()}}`"
                ),
            ));
        }
        Ok((name, parens))
    }
}
