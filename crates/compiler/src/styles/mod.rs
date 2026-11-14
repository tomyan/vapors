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
mod walk;

use std::collections::BTreeMap;

pub use facts::ElementFacts;
pub use report::report;

use vapors_css::analysis::{KeyWinners, SheetId};
use vapors_css::ast::Stylesheet;

use crate::CompileError;
use crate::ast::Component;
use crate::build::Styles;
use crate::source::Source;
use walk::Walker;

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
    /// The same per custom property the key's rules declare, or a
    /// template parent's do.
    pub resolved_custom: BTreeMap<String, Resolved>,
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
    /// Whether the crate's other sheets were given (`wisp-check` gives
    /// none; the build does): without them a rule of another file can
    /// still override a winner reported constant here.
    pub others_given: bool,
    /// Every template element in source order.
    pub elements: Vec<ElementStyles>,
}

/// Where an element sits, for values taken from the parent.
#[derive(Clone, Copy)]
pub(super) enum Parent {
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
/// A literal `style` that does not parse, a `<style>` the build
/// refuses (`style_check`), an attribute the key refuses
/// (`codegen::key_parts`), or sheets the analysis refuses, naming the
/// position.
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
        others_given: !others.is_empty(),
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
