//! Each component's own anchor key, for a tag that carries no `class`,
//! `class:` or `style` and so gets none from the compiler: the scene
//! tag and nothing else. Every such anchor of one component shares one
//! key, and so one computed style per parent style.

use vapors_dom::StyleKey;

/// Where an own key's anchor came from, for messages.
const SITE: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/src/own_keys.rs (a tag with no class or style)"
);

const fn own(tag: &'static str) -> StyleKey {
    StyleKey {
        site: SITE,
        scope: None,
        tag,
        classes: &[],
        toggles: &[],
        style: None,
    }
}

pub(crate) static SCENE: StyleKey = own("scene");
pub(crate) static GROUP: StyleKey = own("group");
pub(crate) static MESH: StyleKey = own("mesh");
pub(crate) static CAMERA: StyleKey = own("camera");
