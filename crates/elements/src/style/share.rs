//! The share table (analysis PR 2): one computed style per share key,
//! taken by every keyed element whose key, parent style, states,
//! toggled classes and selected attributes are equal. The parent is
//! identified by the address of its kept style, which the entry pins so
//! the address cannot be reused while the entry lives; a keyed
//! shareable parent's address stands for its own share key, so the
//! chain encodes every ancestor's inputs up to the nearest unkeyed
//! element, which is then the same node.

use std::collections::HashMap;
use std::rc::Rc;

use vapors_css::tree::States;

use super::Sheets;
use super::styles::Counted;

/// What one shared style is for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ShareKey {
    /// The template key's address.
    pub(crate) key: usize,
    /// The parent's kept style's address; 0 without a parent style.
    pub(crate) parent: usize,
    pub(crate) states: States,
    /// Which `class:` directives are on.
    pub(crate) toggles: u64,
    /// The element's attributes any sheet selects by name, with values.
    pub(crate) attributes: Box<[(String, String)]>,
}

struct Entry {
    style: Rc<Counted>,
    /// Pinned so `ShareKey::parent` stays this parent's address.
    _parent: Option<Rc<Counted>>,
}

/// The shared styles of one `Kept`, and which keys may share at all
/// under the sheets it was made for.
#[derive(Default)]
pub(crate) struct Shared {
    entries: HashMap<ShareKey, Entry>,
    /// By key address: whether no candidate rule of the key reads
    /// siblings or descendants (`restyle_share`).
    pub(crate) shareable: HashMap<usize, bool>,
}

impl Shared {
    /// The style kept for `key`, if any.
    pub(crate) fn take(&self, key: &ShareKey) -> Option<Rc<Counted>> {
        self.entries.get(key).map(|entry| Rc::clone(&entry.style))
    }

    /// Keeps `style` for `key`, pinning `parent`.
    pub(crate) fn insert(
        &mut self,
        key: ShareKey,
        style: Rc<Counted>,
        parent: Option<Rc<Counted>>,
    ) {
        self.entries.insert(
            key,
            Entry {
                style,
                _parent: parent,
            },
        );
    }

    /// Drops every entry no element holds any more.
    pub(crate) fn sweep(&mut self) {
        self.entries
            .retain(|_, entry| Rc::strong_count(&entry.style) > 1);
    }
}

/// What the share path asks the sheets.
impl Sheets {
    /// The rules any element of a template key can match
    /// (`Cascade::rules_for_key`).
    pub(crate) fn rules_for_key(
        &self,
        tag: &str,
        classes: &[&str],
    ) -> impl Iterator<Item = &vapors_css::ast::Rule> {
        self.cascade.rules_for_key(tag, classes)
    }

    /// Every attribute name a selector of any sheet reads.
    pub(crate) fn selected_attributes(&self) -> &std::collections::BTreeSet<String> {
        self.cascade.selected_attributes()
    }
}
