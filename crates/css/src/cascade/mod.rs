//! The cascade: stylesheets by origin in, computed styles per element out.
//!
//! Cascade order (Cascade Level 4 section 6): origin and importance
//! (user agent normal, author normal, author important, user agent
//! important), then specificity, then source order. Unmatched properties
//! inherit when the table says so, else take their initial value.

mod collect;
mod compute;
mod compute_resolve;
mod computed;
mod conditions;
mod media;
mod media_feature;
mod media_items;
mod media_range;
mod restyle;
mod supports;

pub use computed::ComputedStyle;
pub use restyle::RestyleScope;

pub(crate) use compute::resolve_for;

use crate::Result;
use crate::ast::{Keyframes, Rule, Stylesheet};
use crate::selector::Element;
use crate::value::ColorScheme;
use conditions::Conditions;

/// Where a stylesheet comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// The element layer's defaults.
    UserAgent,
    /// The component's `<style>` and any sheet the app adds.
    Author,
}

/// What the caller knows that `@media` and the relative units need.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Environment {
    /// Viewport width in px.
    pub viewport_width: f64,
    /// Viewport height in px.
    pub viewport_height: f64,
    /// The colour scheme for `light-dark()` and `prefers-color-scheme`.
    pub color_scheme: ColorScheme,
    /// `prefers-reduced-motion: reduce` when true.
    pub reduced_motion: bool,
    /// Device pixels per CSS pixel, for `resolution`.
    pub resolution: f64,
    /// The font size the root inherits from (the user agent default).
    pub root_font_size: f64,
    /// The advance of `0` as a fraction of the font size, for `ch`, until
    /// the text layer supplies the real measurement.
    pub zero_advance_em: f64,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            viewport_width: 0.0,
            viewport_height: 0.0,
            color_scheme: ColorScheme::Light,
            reduced_motion: false,
            resolution: 1.0,
            root_font_size: 16.0,
            zero_advance_em: 0.5,
        }
    }
}

/// Check one at-rule condition at parse time.
pub(crate) fn validate_condition(condition: &crate::ast::Condition) -> Result<()> {
    Conditions::parse(std::slice::from_ref(condition)).map(|_| ())
}

/// One rule with its conditions parsed, in cascade order.
pub(crate) struct CascadeRule {
    pub(crate) origin: Origin,
    pub(crate) rule: Rule,
    pub(crate) conditions: Conditions,
    /// Position in the combined source order.
    pub(crate) order: usize,
}

/// Stylesheets by origin, ready to compute styles.
pub struct Cascade {
    rules: Vec<CascadeRule>,
    keyframes: Vec<Keyframes>,
}

impl Default for Cascade {
    fn default() -> Self {
        Self::new()
    }
}

impl Cascade {
    /// An empty cascade.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            keyframes: Vec::new(),
        }
    }

    /// Add a user-agent stylesheet.
    #[must_use]
    pub fn user_agent(self, sheet: Stylesheet) -> Self {
        self.add(Origin::UserAgent, sheet)
    }

    /// Add an author stylesheet.
    #[must_use]
    pub fn author(self, sheet: Stylesheet) -> Self {
        self.add(Origin::Author, sheet)
    }

    fn add(mut self, origin: Origin, sheet: Stylesheet) -> Self {
        for rule in sheet.rules {
            let conditions =
                Conditions::parse(&rule.conditions).expect("the parser accepted these conditions");
            self.rules.push(CascadeRule {
                origin,
                order: self.rules.len(),
                rule,
                conditions,
            });
        }
        self.keyframes.extend(sheet.keyframes);
        self
    }

    /// The `@keyframes` block named `name`; the last one declared wins.
    #[must_use]
    pub fn keyframes(&self, name: &str) -> Option<&Keyframes> {
        self.keyframes.iter().rev().find(|k| k.name == name)
    }

    /// The computed style of one element given its parent's.
    ///
    /// # Errors
    /// A `var()` that does not resolve, or a substituted value that fails
    /// the property's grammar, naming the declaration's position.
    pub fn compute<E: Element>(
        &self,
        element: &E,
        parent: Option<&ComputedStyle>,
        env: &Environment,
    ) -> Result<ComputedStyle> {
        let winners = collect::winners(&self.rules, element, env);
        compute::style(&winners, parent, env)
    }

    /// The computed style of `root` and every descendant, in document
    /// order.
    ///
    /// # Errors
    /// As [`Cascade::compute`], for the first element that fails.
    pub fn compute_tree<E: Element>(
        &self,
        root: &E,
        env: &Environment,
    ) -> Result<Vec<(E, ComputedStyle)>> {
        let mut out = Vec::new();
        self.compute_subtree(root, None, env, &mut out)?;
        Ok(out)
    }

    fn compute_subtree<E: Element>(
        &self,
        element: &E,
        parent: Option<&ComputedStyle>,
        env: &Environment,
        out: &mut Vec<(E, ComputedStyle)>,
    ) -> Result<()> {
        let style = self.compute(element, parent, env)?;
        let index = out.len();
        out.push((element.clone(), style));
        for child in element.children() {
            let parent_style = out[index].1.clone();
            self.compute_subtree(&child, Some(&parent_style), env, out)?;
        }
        Ok(())
    }

    /// Which elements a flip of `flag` on one element can restyle. See
    /// `docs/css.md`, "Incremental restyle".
    #[must_use]
    pub fn restyle_scope(&self, flag: crate::tree::States) -> RestyleScope {
        restyle::scope(self.rules.iter().map(|r| &r.rule), flag)
    }
}
