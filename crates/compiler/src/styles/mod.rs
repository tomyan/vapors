//! The build-time styles analysis of one component
//! (`docs/plans/build-time-styles.md`, row B1): for each template
//! element, every candidate rule of the user-agent sheet, the
//! component's own scoped sheet and the crate's other sheets judged
//! never / always / conditional, and per longhand and custom property
//! a constant or variable winner (`vapors_css::analysis`). A value a
//! winner takes from its parent is resolved through the template's own
//! parents; at a component's root, inside a snippet, or under a
//! component tag it is variable, since the mount site decides.

mod facts;
mod report;

pub use facts::ElementFacts;
pub use report::report;

use vapors_css::analysis::{Constancy, KeyWinners, Sheet, SheetId, Winner, analyse as analyse_key};
use vapors_css::ast::Stylesheet;

use crate::CompileError;
use crate::ast::{Component, Node};
use crate::build::Styles;
use crate::source::Source;

/// One element's analysis.
#[derive(Debug)]
pub struct ElementStyles {
    /// 1-based line and column of the element.
    pub line: usize,
    /// The column.
    pub col: usize,
    /// What the element fixes.
    pub facts: ElementFacts,
    /// Its rules and winners.
    pub winners: KeyWinners,
    /// Per longhand, by `PropertyId::index`: whether the computed value
    /// is the same for every element of this key in every situation.
    pub resolved: Vec<Resolved>,
}

/// A winner's value once parents are followed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// Needs no context.
    Constant,
    /// Decided at run time, for the reason named.
    Variable(&'static str),
}

/// The component's analysis.
#[derive(Debug)]
pub struct Analysis {
    /// The sheets the elements' places index: UA, the own sheet, others.
    pub sheets: Vec<(SheetId, Stylesheet)>,
    /// Every template element in source order.
    pub elements: Vec<ElementStyles>,
}

/// Where an element sits, for values taken from the parent.
#[derive(Clone, Copy)]
enum Parent {
    /// A root of the component: the mount site's ancestors decide.
    Root,
    /// Under the element at this index of the analysis.
    Element(usize),
    /// Inside a snippet or a component tag's children: placed by code
    /// the analysis does not follow.
    Unknown,
}

/// Analyses `component` with the crate's other sheets `others`, each
/// as the runtime registers it (scoped sheets already scoped).
///
/// # Errors
/// A literal `style` that does not parse, or a `<style>` the build
/// refuses (`style_check`), naming the position.
pub fn analyse(
    component: &Component,
    others: &[(SheetId, Stylesheet)],
) -> Result<Analysis, CompileError> {
    let source = Source::new(&component.path, &component.text);
    let mut sheets = vec![(SheetId::UserAgent, vapors_css::user_agent::sheet())];
    if let Some(section) = &component.style {
        sheets.push(own_sheet(component, section, &source)?);
    }
    sheets.extend(others.iter().cloned());
    let scope = component.style.as_ref().and(component.scope.as_deref());
    let mut walker = Walker {
        source: &source,
        scope,
        sheets: &sheets,
        own: 1 + usize::from(component.style.is_some() && scope.is_some()),
        elements: Vec::new(),
    };
    walker.walk(&component.template.children, Parent::Root)?;
    Ok(Analysis {
        elements: walker.elements,
        sheets,
    })
}

/// The component's `<style>` as the runtime registers it: scoped
/// under its scope, or a global sheet named by the file.
fn own_sheet(
    component: &Component,
    section: &crate::ast::Section,
    source: &Source,
) -> Result<(SheetId, Stylesheet), CompileError> {
    Ok(match &component.scope {
        Some(scope) => {
            let sheet = crate::style_check::checked(source, section, Styles::Scoped)?;
            let scoped = vapors_css::scope::scoped(sheet, crate::SCOPE_ATTRIBUTE, scope);
            (SheetId::Scope(scope.clone()), scoped)
        }
        None => (
            SheetId::Global(component.path.clone()),
            crate::style_check::checked(source, section, Styles::Global)?,
        ),
    })
}

struct Walker<'a> {
    source: &'a Source,
    scope: Option<&'a str>,
    sheets: &'a [(SheetId, Stylesheet)],
    /// How many leading sheets are the key's own (UA, and the scoped
    /// own sheet when there is one): judged `Always` where they fit.
    own: usize,
    elements: Vec<ElementStyles>,
}

impl Walker<'_> {
    fn walk(&mut self, nodes: &[Node], parent: Parent) -> Result<(), CompileError> {
        for node in nodes {
            match node {
                Node::Element(element) => {
                    let facts = ElementFacts::of(element, self.scope, self.source)?;
                    let index = self.push(facts, element.offset, parent);
                    self.walk(&element.children, Parent::Element(index))?;
                }
                Node::If(block) => {
                    self.walk(&block.then, parent)?;
                    if let Some(otherwise) = &block.otherwise {
                        self.walk(otherwise, parent)?;
                    }
                }
                Node::For(block) => self.walk(&block.children, parent)?,
                Node::Snippet(snippet) => self.walk(&snippet.children, Parent::Unknown)?,
                Node::Component(tag) => self.walk(&tag.children, Parent::Unknown)?,
                Node::Text(_) | Node::Render(_) => {}
            }
        }
        Ok(())
    }

    /// Analyses one element and records it; its index.
    fn push(&mut self, facts: ElementFacts, offset: usize, parent: Parent) -> usize {
        let sheets: Vec<Sheet<'_>> = (self.sheets.iter().enumerate())
            .map(|(index, (id, rules))| Sheet {
                id,
                rules,
                own: index < self.own,
            })
            .collect();
        let winners = facts.with_key(|key| analyse_key(&sheets, key, &facts.inline));
        let resolved = (winners.longhands.iter().enumerate())
            .map(|(index, winner)| self.resolve(winner, index, parent))
            .collect();
        let (line, col) = self.source.position(offset);
        self.elements.push(ElementStyles {
            line,
            col,
            facts,
            winners,
            resolved,
        });
        self.elements.len() - 1
    }

    fn resolve(&self, winner: &Winner, longhand: usize, parent: Parent) -> Resolved {
        let value = match winner {
            Winner::Constant { value, .. } | Winner::Undeclared { value } => *value,
            Winner::Variable { .. } => return Resolved::Variable("conditional rules"),
        };
        match (value, parent) {
            (Constancy::Constant, _) => Resolved::Constant,
            (Constancy::Variable, _) => Resolved::Variable("the value reads its context"),
            (Constancy::FromParent, Parent::Element(index)) => {
                self.elements[index].resolved[longhand]
            }
            (Constancy::FromParent, Parent::Root) => Resolved::Variable("the mount site's parent"),
            (Constancy::FromParent, Parent::Unknown) => {
                Resolved::Variable("placed by a snippet or a component")
            }
        }
    }
}
