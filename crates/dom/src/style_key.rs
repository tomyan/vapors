//! What a `.wisp` template says about one element that selectors can
//! read: its scope, tag, literal classes, literal `style`, every other
//! literal attribute, and the names its `class:` directives toggle. An
//! expression attribute (`id={expr}`, the components' ARIA ids) is not
//! in it: the tree reads it as it reads any attribute set at run time.
//! The compiler emits one `static` per template element;
//! [`crate::Dom::create_keyed_element`] takes it, sets those
//! attributes from it, and from then on refuses `set_attribute` of any
//! of them, so what the key says stays true: a tree can share one
//! computed style between elements with the same key (the elements
//! crate, analysis PR 2), and the build-time analysis can judge a
//! selector on a literal attribute (`vapors_css::analysis`; the
//! analysable-sources decision, 2026-11-11).

use crate::SCOPE_ATTRIBUTE;

/// One template element's key. Compared by address, not content: the
/// compiler emits a distinct `static` per element, and two elements
/// with equal content are still two template sites.
#[derive(Debug)]
pub struct StyleKey {
    /// `file:line:column` of the element in its `.wisp`, for messages.
    pub site: &'static str,
    /// The scope the component's `<style>` is added under; `None` in a
    /// component without one.
    pub scope: Option<&'static str>,
    /// The element's tag.
    pub tag: &'static str,
    /// The literal `class` names, in template order. Each is one name:
    /// not empty, no whitespace ([`StyleKey::shape_refusal`]).
    pub classes: &'static [&'static str],
    /// Each `class:` directive's name, in template order: at most
    /// [`MAX_TOGGLES`] (a tree keeps which are on as bits), each listed
    /// once and none a literal class too.
    pub toggles: &'static [&'static str],
    /// The literal `style` text.
    pub style: Option<&'static str>,
    /// Every other literal attribute, (name, value) in template order:
    /// not `class`, `style` or the scope attribute, each name once
    /// ([`StyleKey::shape_refusal`]). Listeners, bindings and
    /// expression attributes are not attributes the key carries.
    pub attributes: &'static [(&'static str, &'static str)],
}

/// How many `class:` directives one element may have.
pub const MAX_TOGGLES: usize = 64;

impl PartialEq for StyleKey {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for StyleKey {}

/// The bit of the directive at `index`; 0 past [`MAX_TOGGLES`], which
/// a key with that many directives is refused for
/// ([`StyleKey::shape_refusal`]).
fn bit(index: usize) -> u64 {
    u32::try_from(index)
        .ok()
        .and_then(|index| 1u64.checked_shl(index))
        .unwrap_or(0)
}

impl StyleKey {
    /// The attributes an element takes from the key at creation, with
    /// no directive on: the scope attribute, `class` (when the key has
    /// classes or directives), `style`, each when present, then every
    /// other literal attribute.
    #[must_use]
    pub fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(scope) = self.scope {
            out.push((SCOPE_ATTRIBUTE, scope.to_owned()));
        }
        if !self.classes.is_empty() || !self.toggles.is_empty() {
            out.push(("class", self.class_attribute(0)));
        }
        if let Some(style) = self.style {
            out.push(("style", style.to_owned()));
        }
        for (name, value) in self.attributes {
            out.push((name, (*value).to_owned()));
        }
        out
    }

    /// The `class` text: the literal names, then each directive whose
    /// bit is set in `mask`, in key order.
    #[must_use]
    pub fn class_attribute(&self, mask: u64) -> String {
        let on: Vec<(&str, bool)> = (self.toggles.iter().enumerate())
            .map(|(index, name)| (*name, mask & bit(index) != 0))
            .collect();
        crate::class_names(&self.classes.join(" "), &on)
    }

    /// `mask` with each named directive turned on or off.
    ///
    /// # Errors
    /// A name the key does not list, with the fix.
    pub fn toggled(&self, mut mask: u64, toggles: &[(&str, bool)]) -> Result<u64, String> {
        for (name, on) in toggles {
            let index = self.toggles.iter().position(|t| t == name);
            let bit = bit(index.ok_or_else(|| self.unlisted(name))?);
            if *on {
                mask |= bit;
            } else {
                mask &= !bit;
            }
        }
        Ok(mask)
    }
}
