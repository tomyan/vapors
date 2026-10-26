//! Styles: the user-agent sheet and the global author sheets in one
//! cascade, computed for every element; then what each computed value
//! means to layout, text and paint.

mod box_align;
mod box_style;
mod cursor;
mod decoration;
mod features;
mod grid_style;
mod honoured;
mod inline;
mod outline;
mod overflow;
mod position;
mod scope;
mod scrollbar;
mod sizes;
mod styles;
mod text_style;
mod used;
mod values;

use vapors_css::cascade::{Cascade, ComputedStyle, Environment};

use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::css_element::ElementRef;
use crate::error::ElementError;

pub(crate) use box_style::box_style;
pub use cursor::{ALL_CURSORS, Cursor};
pub(crate) use decoration::BoxShadow;
pub(crate) use overflow::to_viewport;
pub(crate) use scrollbar::Scrollbar;
pub(crate) use styles::Styles;
pub(crate) use text_style::Face;
pub(crate) use text_style::TextStyle;
pub(crate) use used::{Context, Used, used};
pub(crate) use values::{Refusal, Styles as ElementStyles};

const UA_SHEET: &str = include_str!("ua.css");

/// Why a `:global()` below a selector's top level is refused.
const NESTED_GLOBAL: &str = ":global() leaves a top-level compound unscoped; inside :is(), :not(), :where() or :has() it would be ignored, so write that selector without it";

/// The stylesheets, in cascade order, and their names for errors; the
/// component scopes added, and the scoped styles that did not parse.
pub(crate) struct Sheets {
    cascade: Cascade,
    /// What the selectors use that widens a restyle.
    pub(crate) features: features::Features,
    names: Vec<String>,
    /// Each scope added, with the style it was added with.
    scopes: std::collections::HashMap<String, String>,
    refused: Vec<ElementError>,
}

impl Sheets {
    pub(crate) fn new() -> Self {
        let ua = vapors_css::parse(UA_SHEET).unwrap_or_else(|error| {
            panic!(
                "vapors-elements' ua.css does not parse at {}: {}",
                error.position, error.message
            )
        });
        Self {
            features: features::Features::default().with(&ua),
            cascade: Cascade::new().user_agent(ua),
            names: vec!["ua.css".to_owned()],
            scopes: std::collections::HashMap::new(),
            refused: Vec::new(),
        }
    }

    pub(crate) fn add_author(&mut self, name: &str, text: &str) -> Result<(), ElementError> {
        let refuse = |error| ElementError::Css {
            sheet: name.to_owned(),
            error,
        };
        let sheet = vapors_css::parse(text).map_err(refuse)?;
        if let Some(position) = scope::first_global(&sheet) {
            return Err(refuse(vapors_css::Error::new(
                position,
                ":global() only means something in a component's <style>, where the rest is scoped; in this sheet every rule is already global, so write the selector without it",
            )));
        }
        self.features = self.features.with(&sheet);
        self.cascade = std::mem::take(&mut self.cascade).author(sheet);
        self.names.push(name.to_owned());
        Ok(())
    }

    /// A component's style under its `scope`, added once however often
    /// it is mounted. One that does not parse is kept as a refusal for
    /// every update (see [`Sheets::refused`]): `Dom::add_style` returns
    /// nothing, as the generated code's other calls do.
    pub(crate) fn add_scoped(&mut self, scope: &str, text: &str) {
        let refuse = |error| ElementError::Css {
            sheet: scope.to_owned(),
            error,
        };
        match self.scopes.get(scope) {
            Some(added) if added == text => return,
            Some(_) => {
                return self.refused.push(refuse(vapors_css::Error::new(
                    vapors_css::Position { line: 1, column: 1 },
                    format!("scope `{scope}` was already added with another style: two components share a scope; give their files different paths"),
                )));
            }
            None => drop(self.scopes.insert(scope.to_owned(), text.to_owned())),
        }
        let sheet = match vapors_css::parse(text) {
            Ok(sheet) => sheet,
            Err(error) => return self.refused.push(refuse(error)),
        };
        if let Some(position) = scope::first_nested_global(&sheet) {
            return self
                .refused
                .push(refuse(vapors_css::Error::new(position, NESTED_GLOBAL)));
        }
        let sheet = scope::scoped(sheet, scope);
        self.features = self.features.with(&sheet);
        self.cascade = std::mem::take(&mut self.cascade).author(sheet);
        self.names.push(scope.to_owned());
    }

    /// Which elements a flip of `flag` restyles, as the sheets use it.
    pub(crate) fn restyle_scope(
        &self,
        flag: vapors_css::tree::States,
    ) -> vapors_css::cascade::RestyleScope {
        self.cascade.restyle_scope_of_selectors(flag)
    }

    /// The scoped styles that did not parse.
    pub(crate) fn refused(&self) -> &[ElementError] {
        &self.refused
    }

    /// The computed style of `node`, whose parent's is `parent`.
    pub(crate) fn compute_element(
        &self,
        arena: &Arena,
        node: NodeId,
        parent: Option<&ComputedStyle>,
        env: &Environment,
    ) -> Result<ComputedStyle, ElementError> {
        self.cascade
            .compute(&ElementRef::new(arena, node), parent, env)
            .map_err(|error| ElementError::Css {
                sheet: self.names.join(" or "),
                error,
            })
    }
}

impl Sheets {
    /// The name a sheet was added under, by its cascade order (0 is the
    /// user-agent sheet).
    pub(crate) fn name(&self, index: usize) -> &str {
        if index == vapors_css::cascade::INLINE_SHEET {
            return "style attribute";
        }
        self.names
            .get(index)
            .map_or("an unnamed sheet", String::as_str)
    }
}

/// Every property at its initial value: what the deferred-property
/// check compares against.
pub(crate) fn initial_style(arena: &Arena, root: NodeId, env: &Environment) -> ComputedStyle {
    Cascade::new()
        .compute(&ElementRef::new(arena, root), None, env)
        .expect("an empty cascade computes initial values without var()")
}
