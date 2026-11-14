//! What a 3D scene node takes from its anchor element's computed style
//! (docs/plans/css-for-3d.md, C3): the tree reports it through
//! [`Dom::watch_appearance`](crate::Dom::watch_appearance), and the scene
//! component writes it to its node.

/// A mesh's appearance: its anchor's computed `color` and
/// `visibility`. Opaque: a tree refuses a translucent `color` or an
/// `opacity` under 1 on a watched anchor until the scene draws
/// translucent meshes (C5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appearance {
    /// sRGB-encoded red, green and blue, each from 0 to 1.
    pub rgb: [f32; 3],
    /// `visibility: visible`.
    pub visible: bool,
}

/// Called with an anchor's appearance at the first update after the
/// watch and whenever an update changes it. An `Err` refuses that
/// update, naming the anchor and the message; the next update calls it
/// again.
pub type AppearanceSink = Box<dyn FnMut(Appearance) -> Result<(), String>>;
