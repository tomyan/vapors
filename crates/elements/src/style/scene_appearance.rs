//! What a watched mesh anchor gives its node (`Dom::watch_appearance`,
//! docs/plans/css-for-3d.md C3): its colour's channels and visibility.
//! The scene draws opaque meshes only until the blended pass (C5), so a
//! colour whose alpha times `opacity` is under 1 is refused at the mesh,
//! naming the declaration when the mesh has one; never drawn opaque.

use vapors_css::property::PropertyId;
use vapors_css::property::longhands::{COLOR, OPACITY};
use vapors_dom::Appearance;

use super::scene_used::SceneUsed;
use super::values::{Refusal, Styles};

impl SceneUsed {
    /// The colour's channels and the visibility; [`refuse_translucent`]
    /// checks the alpha this leaves out.
    pub(crate) fn appearance(&self) -> Appearance {
        let colour = self.colour;
        Appearance {
            rgb: [colour.r, colour.g, colour.b],
            visible: self.visible,
        }
    }
}

/// A refusal of each of `opacity` and `color` that is under 1 when
/// `used` would draw its mesh translucent, so one update names both.
pub(crate) fn refuse_translucent(used: &SceneUsed, styles: Styles<'_>) -> Vec<Refusal> {
    let alpha = used.colour.a * used.opacity;
    if alpha >= 1.0 {
        return Vec::new();
    }
    let under = [(OPACITY, used.opacity), (COLOR, used.colour.a)];
    (under.into_iter())
        .filter(|(_, value)| *value < 1.0)
        .map(|(property, _)| translucent(property, alpha, styles))
        .collect()
}

fn translucent(property: PropertyId, alpha: f32, styles: Styles<'_>) -> Refusal {
    let own = styles.own;
    Refusal {
        property: property.name(),
        value: own.get(property).to_css(),
        reason: format!(
            "draws a mesh translucent (alpha {alpha}); the scene draws meshes opaque until its blended pass arrives in C5 (docs/plans/css-for-3d.md)"
        ),
        declared: own.declared_in(property).zip(own.declared_at(property)),
    }
}
