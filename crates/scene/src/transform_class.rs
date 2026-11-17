//! What a frame's composed transform to the target is
//! (`docs/plans/flat-scene.md` section 4.3), which decides how its
//! items are painted: snapped to the device pixel grid under a whole
//! translation, unsnapped under any other.

/// The class of the composed transform from a frame to the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransformClass {
    /// A translation by whole device pixels.
    WholeTranslate,
    /// Any other affine: a fractional translation, a scale, a rotation,
    /// a skew.
    Affine,
    /// A perspective or 3D rotation: w differs from 1.
    Projective,
}

impl TransformClass {
    /// Whether items under it are snapped to the device pixel grid, as
    /// Chromium snaps a box's edges under a whole translation.
    #[must_use]
    pub fn snaps(self) -> bool {
        self == Self::WholeTranslate
    }
}

#[cfg(test)]
#[path = "transform_class_tests.rs"]
mod tests;
