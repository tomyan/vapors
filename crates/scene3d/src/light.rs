//! Lights: what a light node is, the scene's ambient level, and the
//! limit one lights uniform sets (`docs/plans/lighting.md`, LT2).

use crate::error::finite_scalar;
use crate::{Colour, NodeId, NodeKind, Scene, SceneError, StoreError};

/// The most lights in one scene: the size of the lights uniform.
pub const MAX_LIGHTS: usize = 8;

/// What a light node shines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light {
    /// Parallel rays along the node's -Z, from infinitely far.
    Directional {
        /// Its colour.
        colour: Colour,
        /// A multiplier on the colour, 0 or more.
        intensity: f32,
    },
    /// From the node's position, falling off smoothly to zero at `range`.
    Point {
        /// Its colour.
        colour: Colour,
        /// A multiplier on the colour, 0 or more.
        intensity: f32,
        /// The distance at which it reaches zero, above 0.
        range: f32,
    },
}

impl Light {
    /// Refuses an intensity that is negative or not finite, or a range
    /// that is not above 0.
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`] or [`SceneError::OutOfRange`].
    pub fn check(&self) -> Result<(), SceneError> {
        match *self {
            Self::Directional { intensity, .. } => {
                check_intensity("Light::Directional intensity", intensity)
            }
            Self::Point {
                intensity, range, ..
            } => {
                check_intensity("Light::Point intensity", intensity)?;
                finite_scalar("Light::Point range", range)?;
                if range > 0.0 {
                    return Ok(());
                }
                Err(SceneError::OutOfRange {
                    field: "Light::Point range",
                    value: range,
                    rule: "a distance greater than 0",
                })
            }
        }
    }
}

fn check_intensity(field: &'static str, intensity: f32) -> Result<(), SceneError> {
    finite_scalar(field, intensity)?;
    if intensity >= 0.0 {
        return Ok(());
    }
    Err(SceneError::OutOfRange {
        field,
        value: intensity,
        rule: "an intensity of 0 or more; 0 turns the light off",
    })
}

/// The light every surface gets from every direction: the scene root's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ambient {
    colour: Colour,
    intensity: f32,
}

impl Ambient {
    /// No ambient light: what a scene starts with.
    pub const NONE: Self = Self {
        colour: Colour::BLACK,
        intensity: 0.0,
    };

    /// `colour` times `intensity`.
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`] or [`SceneError::OutOfRange`] for the
    /// intensity.
    pub fn new(colour: Colour, intensity: f32) -> Result<Self, SceneError> {
        check_intensity("Ambient intensity", intensity)?;
        Ok(Self { colour, intensity })
    }

    /// Its colour as linear light, times its intensity.
    #[must_use]
    pub fn linear(&self) -> [f32; 3] {
        self.colour.linear().map(|c| c * self.intensity)
    }
}

impl Scene {
    /// Sets the ambient level; the renderer writes the lights uniform
    /// when it changes.
    pub fn set_ambient(&mut self, ambient: Ambient) {
        self.ambient = ambient;
    }

    /// The ambient level.
    #[must_use]
    pub fn ambient(&self) -> Ambient {
        self.ambient
    }

    /// Every light node with its light, parents before children.
    #[must_use]
    pub fn lights(&self) -> Vec<(NodeId, Light)> {
        self.subtree(self.root())
            .into_iter()
            .filter_map(|id| match self.node(id).map(|node| &node.kind) {
                Ok(NodeKind::Light(light)) => Some((id, *light)),
                _ => None,
            })
            .collect()
    }

    /// Refuses a light that fails [`Light::check`], or one past
    /// [`MAX_LIGHTS`]; any other kind passes.
    pub(crate) fn check_light(&self, kind: &NodeKind) -> Result<(), StoreError> {
        let NodeKind::Light(light) = kind else {
            return Ok(());
        };
        light.check().map_err(StoreError::Light)?;
        if self.lights().len() >= MAX_LIGHTS {
            return Err(StoreError::TooManyLights);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "light_tests.rs"]
mod tests;
