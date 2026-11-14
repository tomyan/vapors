//! A scene anchor's used style (docs/plans/css-for-3d.md, 4.3): the
//! appearance its node reads, in place of a box `Used`; a scene anchor
//! builds no box and paints nothing. Each author declaration of a
//! longhand the tag does not read (vapors-css's scene property set) is
//! refused naming what to use instead. Only declarations are checked: a
//! value inherited from HTML above the scene is not the node's to
//! refuse, and the user-agent sheet's `display: none` is the anchor's.

use vapors_css::property::PropertyId;
use vapors_css::property::longhands::{CURSOR, OPACITY, POINTER_EVENTS, VISIBILITY};
use vapors_css::property::scene::{SceneTag, instead};
use vapors_css::value::Value;
use vapors_dom::StyleKey;
use vapors_scene::Color;

use super::cursor::{Cursor, cursor};
use super::values::{Reader, Refusal, Styles, keyword, shape};

/// What a scene node reads from its anchor's computed style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneUsed {
    /// `color`, alpha kept: a mesh's colour.
    pub colour: Color,
    /// `opacity` on a `mesh`; 1 on tags that do not read it.
    pub opacity: f32,
    /// `visibility: visible`.
    pub visible: bool,
    /// `pointer-events: auto`.
    pub takes_pointer: bool,
    /// `cursor` on a `mesh`; the default on tags that do not read it.
    pub cursor: Cursor,
}

/// Where a scene anchor is and which sheet is the user agent's.
pub(crate) struct SceneContext {
    pub(crate) tag: SceneTag,
    /// The anchor's key, whose site the refusals name.
    pub(crate) key: Option<&'static StyleKey>,
    /// The user-agent sheet's index in the cascade.
    pub(crate) user_agent: usize,
}

/// The used style of a scene anchor whose computed styles are `styles`.
pub(crate) fn scene_used(
    styles: Styles<'_>,
    context: &SceneContext,
) -> Result<SceneUsed, Vec<Refusal>> {
    let mut refusals = refuse_unread(styles, context);
    let mut reader = Reader::new(styles);
    let mesh = context.tag == SceneTag::Mesh;
    let used = SceneUsed {
        colour: styles.own.color(),
        opacity: if mesh {
            reader.read(OPACITY, opacity, 1.0)
        } else {
            1.0
        },
        visible: reader.read(VISIBILITY, |v| keyword(v).map(|k| k == "visible"), true),
        takes_pointer: reader.read(POINTER_EVENTS, |v| keyword(v).map(|k| k == "auto"), true),
        cursor: if mesh {
            reader.read(CURSOR, cursor, Cursor::Default)
        } else {
            Cursor::Default
        },
    };
    refusals.extend(reader.refusals);
    if refusals.is_empty() {
        Ok(used)
    } else {
        Err(refusals)
    }
}

/// A refusal for each author declaration `context.tag` does not read,
/// and for each it reads that no node implements yet.
fn refuse_unread(styles: Styles<'_>, context: &SceneContext) -> Vec<Refusal> {
    PropertyId::all()
        .filter(|&id| {
            let sheet = styles.own.declared_in(id);
            sheet.is_some_and(|sheet| sheet != context.user_agent)
        })
        .filter_map(|id| reason(id, context).map(|reason| (id, reason)))
        .filter_map(|(id, reason)| styles.refusal(id, reason))
        .collect()
}

/// Why `id`, declared on a scene anchor, is refused; `None` when its
/// node reads it.
fn reason(id: PropertyId, context: &SceneContext) -> Option<String> {
    let site = context
        .key
        .map(|key| format!(" (the <{}> at {})", key.tag, key.site))
        .unwrap_or_default();
    let tag = context.tag.name();
    if !context.tag.reads(id) {
        return Some(format!(
            "is not read by a `{tag}` scene node{site}: {} (docs/plans/css-for-3d.md, section 3)",
            instead(id)
        ));
    }
    is_motion(id).then(|| format!(
        "is not implemented on scene nodes yet{site}: transitions and animations on a `{tag}` arrive in C7 (docs/plans/css-for-3d.md)"
    ))
}

/// A `transition-*` or `animation-*` longhand.
fn is_motion(id: PropertyId) -> bool {
    let name = id.name();
    name.starts_with("transition-") || name.starts_with("animation-")
}

/// `opacity` as a fraction.
#[allow(clippy::cast_possible_truncation)] // a fraction from 0 to 1
fn opacity(value: &Value) -> Result<f32, String> {
    match value {
        Value::Number(n) => Ok(*n as f32),
        Value::Percentage(p) => Ok((*p / 100.0) as f32),
        _ => Err(shape("a number or a percentage")),
    }
}
