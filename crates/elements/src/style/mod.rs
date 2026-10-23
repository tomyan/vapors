//! Styles: the user-agent sheet and the global author sheets in one
//! cascade, computed for every element; then what each computed value
//! means to layout, text and paint.

mod box_align;
mod box_style;
mod cursor;
mod decoration;
mod grid_style;
mod honoured;
mod scope;
mod sizes;
mod text_style;
mod used;
mod values;

use vapors_css::cascade::{Cascade, ComputedStyle, Environment};

use vapors_dom::NodeId;

use crate::arena::{Arena, index_of};
use crate::css_element::ElementRef;
use crate::error::ElementError;

pub(crate) use box_style::box_style;
pub use cursor::{ALL_CURSORS, Cursor};
pub(crate) use decoration::BoxShadow;
pub(crate) use text_style::TextStyle;
pub(crate) use used::{Context, Used, used};
pub(crate) use values::{Refusal, Styles as ElementStyles};

const UA_SHEET: &str = include_str!("ua.css");

/// The stylesheets, in cascade order, and their names for errors; the
/// component scopes added, and the scoped styles that did not parse.
pub(crate) struct Sheets {
    cascade: Cascade,
    names: Vec<String>,
    scopes: std::collections::HashSet<String>,
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
            cascade: Cascade::new().user_agent(ua),
            names: vec!["ua.css".to_owned()],
            scopes: std::collections::HashSet::new(),
            refused: Vec::new(),
        }
    }

    pub(crate) fn add_author(&mut self, name: &str, text: &str) -> Result<(), ElementError> {
        let sheet = vapors_css::parse(text).map_err(|error| ElementError::Css {
            sheet: name.to_owned(),
            error,
        })?;
        self.cascade = std::mem::take(&mut self.cascade).author(sheet);
        self.names.push(name.to_owned());
        Ok(())
    }

    /// A component's style under its `scope`, added once however often
    /// it is mounted. One that does not parse is kept as a refusal for
    /// every update (see [`Sheets::refused`]): `Dom::add_style` returns
    /// nothing, as the generated code's other calls do.
    pub(crate) fn add_scoped(&mut self, scope: &str, text: &str) {
        if !self.scopes.insert(scope.to_owned()) {
            return;
        }
        match vapors_css::parse(text) {
            Ok(sheet) => {
                self.cascade =
                    std::mem::take(&mut self.cascade).author(scope::scoped(sheet, scope));
                self.names.push(scope.to_owned());
            }
            Err(error) => self.refused.push(ElementError::Css {
                sheet: scope.to_owned(),
                error,
            }),
        }
    }

    /// The scoped styles that did not parse.
    pub(crate) fn refused(&self) -> &[ElementError] {
        &self.refused
    }

    /// The computed style of every element under `root`, by slot.
    pub(crate) fn compute(
        &self,
        arena: &Arena,
        root: NodeId,
        env: &Environment,
    ) -> Result<Styles, ElementError> {
        let computed = self
            .cascade
            .compute_tree(&ElementRef::new(arena, root), env)
            .map_err(|error| ElementError::Css {
                sheet: self.names.join(" or "),
                error,
            })?;
        let mut styles = Styles(Vec::new());
        styles.0.resize_with(arena.capacity(), || None);
        for (element, style) in computed {
            styles.0[index_of(element.id) as usize] = Some(style);
        }
        Ok(styles)
    }
}

impl Sheets {
    /// The name a sheet was added under, by its cascade order (0 is the
    /// user-agent sheet).
    pub(crate) fn name(&self, index: usize) -> &str {
        self.names
            .get(index)
            .map_or("an unnamed sheet", String::as_str)
    }
}

/// Computed styles by slot; `None` for text nodes and detached nodes.
pub(crate) struct Styles(Vec<Option<ComputedStyle>>);

impl Styles {
    /// The element's style.
    ///
    /// # Panics
    /// When `node` was not under the root when styles were computed: the
    /// caller walked a different tree from the one styled.
    pub(crate) fn of(&self, node: NodeId) -> &ComputedStyle {
        self.0
            .get(index_of(node) as usize)
            .and_then(Option::as_ref)
            .unwrap_or_else(|| {
                panic!("{node:?} has no computed style; it was not in the styled tree")
            })
    }
}

/// Every property at its initial value: what the deferred-property
/// check compares against.
pub(crate) fn initial_style(arena: &Arena, root: NodeId, env: &Environment) -> ComputedStyle {
    Cascade::new()
        .compute(&ElementRef::new(arena, root), None, env)
        .expect("an empty cascade computes initial values without var()")
}
