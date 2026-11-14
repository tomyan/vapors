//! The walk over a component's template: each element analysed under
//! its parent, with values taken from the parent resolved through the
//! template's own elements.

use vapors_css::analysis::{Constancy, Sheet, SheetId, Winner, analyse as analyse_key};
use vapors_css::ast::Stylesheet;

use super::{ElementFacts, ElementStyles, Parent, Resolved};
use crate::CompileError;
use crate::ast::Node;
use crate::source::Source;

pub(super) struct Walker<'a> {
    pub(super) source: &'a Source,
    pub(super) scope: Option<&'a str>,
    pub(super) sheets: &'a [(SheetId, Stylesheet)],
    /// How many leading sheets are the key's own (UA, and the scoped
    /// own sheet when there is one): judged `Always` where they fit.
    pub(super) own: usize,
    pub(super) elements: Vec<ElementStyles>,
}

impl Walker<'_> {
    pub(super) fn walk(&mut self, nodes: &[Node], parent: Parent) -> Result<(), CompileError> {
        nodes
            .iter()
            .try_for_each(|node| self.walk_node(node, parent))
    }

    /// An element is analysed under `parent` and its children under
    /// it; `{if}` branches and `{for}` rows keep the parent (they are
    /// placed at its anchor); a snippet's body and a component tag's
    /// children are placed by code the analysis does not follow.
    fn walk_node(&mut self, node: &Node, parent: Parent) -> Result<(), CompileError> {
        match node {
            Node::Element(element) => {
                let index = self.push(element, parent)?;
                self.walk(&element.children, Parent::Element(index))
            }
            Node::If(block) => {
                self.walk(&block.then, parent)?;
                block
                    .otherwise
                    .as_ref()
                    .map_or(Ok(()), |otherwise| self.walk(otherwise, parent))
            }
            Node::For(block) => self.walk(&block.children, parent),
            Node::Snippet(snippet) => self.walk(&snippet.children, Parent::Unknown),
            Node::Component(tag) => self.walk(&tag.children, Parent::Unknown),
            Node::Text(_) | Node::Render(_) => Ok(()),
        }
    }

    /// Analyses one element and records it; its index.
    fn push(
        &mut self,
        element: &crate::ast::Element,
        parent: Parent,
    ) -> Result<usize, CompileError> {
        let facts = ElementFacts::of(element, self.scope, self.source)?;
        let sheets: Vec<Sheet<'_>> = (self.sheets.iter().enumerate())
            .map(|(index, (id, rules))| Sheet {
                id,
                rules,
                own: index < self.own,
            })
            .collect();
        let winners = facts
            .with_key(|key| analyse_key(&sheets, key, &facts.inline))
            .map_err(|message| self.source.error(element.offset, message))?;
        let resolved = (winners.longhands.iter().enumerate())
            .map(|(index, winner)| self.resolve(winner, parent, |p| p.resolved[index]))
            .collect();
        let resolved_custom = (winners.custom.iter())
            .map(|(name, winner)| {
                let from_parent = |p: &ElementStyles| {
                    p.resolved_custom
                        .get(name)
                        .copied()
                        .unwrap_or(Resolved::Variable(
                            "the parent's custom property is undeclared",
                        ))
                };
                (name.clone(), self.resolve(winner, parent, from_parent))
            })
            .collect();
        let (line, col) = self.source.position(element.offset);
        self.elements.push(ElementStyles {
            line,
            col,
            facts,
            winners,
            resolved,
            resolved_custom,
        });
        Ok(self.elements.len() - 1)
    }

    /// A winner's computed constancy: its own, or its parent's through
    /// `from_parent` when it takes the parent's value.
    fn resolve(
        &self,
        winner: &Winner,
        parent: Parent,
        from_parent: impl Fn(&ElementStyles) -> Resolved,
    ) -> Resolved {
        let value = match winner {
            Winner::Constant { value, .. } | Winner::Undeclared { value } => *value,
            Winner::Variable { .. } => return Resolved::Variable("conditional rules"),
        };
        match (value, parent) {
            (Constancy::Constant, _) => Resolved::Constant,
            (Constancy::Variable, _) => Resolved::Variable("the value reads its context"),
            (Constancy::FromParent, Parent::Element(index)) => {
                match from_parent(&self.elements[index]) {
                    Resolved::Constant => Resolved::Constant,
                    Resolved::Variable(_) => Resolved::Variable("the parent's value is variable"),
                }
            }
            (Constancy::FromParent, Parent::Root) => Resolved::Variable("the mount site's parent"),
            (Constancy::FromParent, Parent::Unknown) => {
                Resolved::Variable("placed by a snippet or a component")
            }
        }
    }
}
