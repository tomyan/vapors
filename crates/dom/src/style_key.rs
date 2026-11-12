//! What a `.wisp` template says about one element that selectors can
//! read: its scope, tag, `id`, literal classes, literal `style` and
//! the names its `class:` directives toggle. The compiler emits one
//! `static` per template element; [`crate::Dom::create_keyed_element`]
//! takes it, sets those attributes from it, and from then on refuses
//! `set_attribute` of any of them, so what the key says stays true
//! and a tree can share one computed style between elements with the
//! same key (the elements crate, analysis PR 2).

use crate::SCOPE_ATTRIBUTE;

/// One template element's key. Compared by address: the compiler emits
/// a distinct `static` per element.
#[derive(Debug, PartialEq, Eq)]
pub struct StyleKey {
    /// `file:line` of the element in its `.wisp`, for messages.
    pub site: &'static str,
    /// The scope the component's `<style>` is added under; `None` in a
    /// component without one.
    pub scope: Option<&'static str>,
    /// The element's tag.
    pub tag: &'static str,
    /// The literal `id`.
    pub id: Option<&'static str>,
    /// The literal `class` names, in template order.
    pub classes: &'static [&'static str],
    /// Each `class:` directive's name, in template order. At most
    /// [`MAX_TOGGLES`]: a tree keeps which are on as bits.
    pub toggles: &'static [&'static str],
    /// The literal `style` text.
    pub style: Option<&'static str>,
}

/// How many `class:` directives one element may have.
pub const MAX_TOGGLES: usize = 64;

impl StyleKey {
    /// The attributes an element takes from the key at creation, with
    /// no directive on: the scope attribute, `class` (when the key has
    /// classes or directives), `id` and `style`, each when present.
    ///
    /// # Panics
    /// More than [`MAX_TOGGLES`] directives.
    #[must_use]
    pub fn attributes(&self) -> Vec<(&'static str, String)> {
        assert!(
            self.toggles.len() <= MAX_TOGGLES,
            "<{}> at {} has {} `class:` directives; at most {MAX_TOGGLES} fit one element",
            self.tag,
            self.site,
            self.toggles.len()
        );
        let mut out = Vec::new();
        if let Some(scope) = self.scope {
            out.push((SCOPE_ATTRIBUTE, scope.to_owned()));
        }
        if !self.classes.is_empty() || !self.toggles.is_empty() {
            out.push(("class", self.class_attribute(0)));
        }
        if let Some(id) = self.id {
            out.push(("id", id.to_owned()));
        }
        if let Some(style) = self.style {
            out.push(("style", style.to_owned()));
        }
        out
    }

    /// The `class` text: the literal names, then each directive whose
    /// bit is set in `mask`, in key order.
    #[must_use]
    pub fn class_attribute(&self, mask: u64) -> String {
        let on: Vec<(&str, bool)> = (self.toggles.iter().enumerate())
            .map(|(bit, name)| (*name, mask & (1 << bit) != 0))
            .collect();
        crate::class_names(&self.classes.join(" "), &on)
    }

    /// `mask` with each named directive turned on or off.
    ///
    /// # Errors
    /// A name the key does not list, with the fix.
    pub fn toggled(&self, mut mask: u64, toggles: &[(&str, bool)]) -> Result<u64, String> {
        for (name, on) in toggles {
            let bit = self.toggles.iter().position(|t| t == name).ok_or_else(|| {
                let listed: Vec<String> = self.toggles.iter().map(|t| format!("`{t}`")).collect();
                format!(
                    "`class:{name}` is not a directive of <{}> at {}, whose directives are {}; add `class:{name}={{bool}}` there",
                    self.tag,
                    self.site,
                    listed.join(", ")
                )
            })?;
            if *on {
                mask |= 1 << bit;
            } else {
                mask &= !(1 << bit);
            }
        }
        Ok(mask)
    }

    /// Why `set_attribute` of `name` is refused on an element with this
    /// key: `None` when it is not one the key fixes.
    #[must_use]
    pub fn refusal(&self, name: &str) -> Option<String> {
        let fix = match name {
            "class" => "toggle a class with `class:name={bool}` there",
            "id" | "style" | SCOPE_ATTRIBUTE => "write it there",
            _ => return None,
        };
        Some(format!(
            "`{name}` on <{}> at {} is set from its template; {fix}",
            self.tag, self.site
        ))
    }
}
