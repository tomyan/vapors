//! Each component's own anchor key, for a tag that carries no `class`,
//! `class:` or `style` and so gets none from the compiler: the scene
//! tag (its props type's `ANCHOR_TAG`) and nothing else. Every such
//! anchor of one component shares one key, and so one computed style
//! per parent style. A key passed in must have that same tag.

use crate::{CameraProps, GroupProps, MeshProps, SceneProps};
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
        attributes: &[],
        style: None,
    }
}

pub(crate) static SCENE: StyleKey = own(SceneProps::ANCHOR_TAG);
pub(crate) static GROUP: StyleKey = own(GroupProps::ANCHOR_TAG);
pub(crate) static MESH: StyleKey = own(MeshProps::ANCHOR_TAG);
pub(crate) static CAMERA: StyleKey = own(CameraProps::ANCHOR_TAG);

/// The key `component`'s anchor is made from: `passed`, or its `own`.
///
/// # Panics
///
/// When `passed` has another tag than `own`, naming the key's site: an
/// anchor with another tag would be laid out and styled as that tag.
pub(crate) fn anchor_key(
    component: &str,
    passed: Option<&'static StyleKey>,
    own: &'static StyleKey,
) -> &'static StyleKey {
    let key = passed.unwrap_or(own);
    assert!(
        key.tag == own.tag,
        "<{component}> was passed the key of a <{}> ({}): its anchor's tag is `{}`; pass a key made with `{component}Props::ANCHOR_TAG`, as the compiler does",
        key.tag,
        key.site,
        own.tag
    );
    key
}
