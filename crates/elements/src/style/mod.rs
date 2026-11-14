//! Styles: the user-agent sheet and the global author sheets in one
//! cascade, computed for every element; then what each computed value
//! means to layout, text and paint.

mod box_align;
mod box_style;
mod cursor;
mod decoration;
mod features;
mod grid_gate;
#[cfg(feature = "grid")]
mod grid_style;

pub(crate) use grid_gate::is_container;
mod honoured;
mod inline;
pub(crate) mod outline;
mod overflow;
mod parts;
mod position;
mod scene_used;
mod scrollbar;
mod share;
mod sibling_deps;
mod sides;
pub(crate) use sibling_deps::SiblingDeps;
mod sizes;
mod styles;
mod surface;
mod text_style;
mod used;
mod used_share;
mod values;

use vapors_css::cascade::{Cascade, ComputedStyle, Environment};

use vapors_dom::{NodeId, SCOPE_ATTRIBUTE};

use crate::arena::Arena;
use crate::css_element::ElementRef;
use crate::error::ElementError;

pub(crate) use box_style::box_style;
pub use cursor::{ALL_CURSORS, Cursor};
pub(crate) use decoration::BoxShadow;
pub(crate) use overflow::to_viewport;
pub(crate) use parts::Parts;
pub use scene_used::SceneUsed;
pub(crate) use scene_used::{SceneContext, scene_used};
pub(crate) use scrollbar::Scrollbar;
pub(crate) use share::{ShareKey, Shared};
pub(crate) use styles::{Counted, Styles};
pub(crate) use text_style::Face;
pub(crate) use text_style::TextStyle;
pub(crate) use used::{Context, Used, used};
pub(crate) use used_share::{SharedUsed, UsedKey};

/// Each element's used style, shared between elements by `SharedUsed`.
pub(crate) type UsedMap = crate::node_map::NodeMap<std::rc::Rc<Used>>;
pub(crate) use values::{Refusal, Styles as ElementStyles, keyword};

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
        let ua = vapors_css::user_agent::sheet();
        // first, at USER_AGENT_SHEET
        Self {
            features: features::Features::default().with(&ua),
            cascade: Cascade::new().user_agent(ua),
            names: vec![USER_AGENT_NAME.to_owned()],
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
        vapors_css::scope::refuse_global(&sheet).map_err(refuse)?;
        self.features = self.features.with(&sheet);
        self.cascade = std::mem::take(&mut self.cascade).author(sheet);
        self.names.push(name.to_owned());
        Ok(())
    }

    /// A component's style under its `scope`, added once however often
    /// it is mounted. One that does not parse is kept as a refusal for
    /// every update (see [`Sheets::refused`]): `Dom::add_style` returns
    /// nothing, as the generated code's other calls do. False when the
    /// scope was already added with this style and nothing changed.
    pub(crate) fn add_scoped(&mut self, scope: &str, text: &str) -> bool {
        if self.scopes.get(scope).is_some_and(|added| added == text) {
            return false;
        }
        self.add_new_scoped(scope, text);
        true
    }

    fn add_new_scoped(&mut self, scope: &str, text: &str) {
        let refuse = |error| ElementError::Css {
            sheet: scope.to_owned(),
            error,
        };
        match self.scopes.get(scope) {
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
        if let Err(error) = vapors_css::scope::refuse_nested_global(&sheet) {
            return self.refused.push(refuse(error));
        }
        let sheet = vapors_css::scope::scoped(sheet, SCOPE_ATTRIBUTE, scope);
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

    /// The selectors of the rules that reach siblings, as written, each
    /// with whether it matches `node` now: what a narrowed restyle that
    /// went wrong missed.
    pub(crate) fn sibling_selectors(&self, arena: &Arena, node: NodeId) -> Vec<(String, bool)> {
        let element = ElementRef::new(arena, node);
        self.cascade
            .rules()
            .filter(|rule| SiblingDeps::of_selector(&rule.selector).any())
            .map(|rule| {
                let text = vapors_css::serialize::component_values(&rule.prelude);
                let matches = vapors_css::selector::matches(&rule.selector, &element);
                (text.trim().to_owned(), matches)
            })
            .collect()
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

/// The user-agent sheet's index in the cascade: `Sheets::new` adds it
/// first.
pub(crate) const USER_AGENT_SHEET: usize = 0;

/// The user-agent sheet's name, which `restyle_scene` checks is the one
/// at [`USER_AGENT_SHEET`].
pub(crate) const USER_AGENT_NAME: &str = "ua.css";

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
