//! The used-style table (build-time styles U2): one `Used` per shared
//! computed style, parent's `Used` and context, taken by every keyed
//! element whose computed style came from the share table. Both
//! addresses in the key are pinned by the entry, as `ShareKey::parent`
//! is, so neither can be reused while the entry lives. Only derivations
//! that succeed are kept: refusals name their element, so a refused
//! element's are built for it alone.

use std::collections::HashMap;
use std::rc::Rc;

use super::styles::Counted;
use super::used::{Context, Used};

/// What one shared `Used` is for. `Context::initial` and
/// `Context::scale` are not in it: one `Kept` fixes both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct UsedKey {
    /// The element's kept computed style's address.
    style: usize,
    /// The parent's kept `Used`'s address; 0 without one.
    parent: usize,
    /// `Context::bits`.
    context: u8,
}

impl UsedKey {
    pub(crate) fn new(style: &Rc<Counted>, parent: Option<&Rc<Used>>, context: &Context) -> Self {
        Self {
            style: Rc::as_ptr(style) as usize,
            parent: parent.map_or(0, |p| Rc::as_ptr(p) as usize),
            context: context.bits(),
        }
    }

    /// Whether this key's addresses are those of `style` and `parent`.
    fn names(&self, style: &Rc<Counted>, parent: Option<&Rc<Used>>) -> bool {
        let parent_address = parent.map_or(0, |p| Rc::as_ptr(p) as usize);
        self.style == Rc::as_ptr(style) as usize && self.parent == parent_address
    }
}

struct Entry {
    used: Rc<Used>,
    /// Pinned so `UsedKey::style` stays this style's address.
    _style: Rc<Counted>,
    /// Pinned so `UsedKey::parent` stays this parent's address.
    _parent: Option<Rc<Used>>,
}

/// The shared used styles of one `Kept`.
#[derive(Default)]
pub(crate) struct SharedUsed {
    entries: HashMap<UsedKey, Entry>,
}

impl SharedUsed {
    /// The used style kept for `key`, if any.
    pub(crate) fn take(&self, key: &UsedKey) -> Option<Rc<Used>> {
        self.entries.get(key).map(|entry| Rc::clone(&entry.used))
    }

    /// Keeps `used` for `key`, pinning `style` and `parent`, which
    /// must be what `key`'s addresses name: a pin of anything else would
    /// let a named address be reused while the entry can still be hit.
    pub(crate) fn insert(
        &mut self,
        key: UsedKey,
        used: Rc<Used>,
        style: Rc<Counted>,
        parent: Option<Rc<Used>>,
    ) {
        assert!(
            key.names(&style, parent.as_ref()),
            "a used-style entry pins a style or parent its key does not name; restyle_used.rs keep_found looked them up apart from find_used's key. Reproduce with `cargo xtask test`"
        );
        let entry = Entry {
            used,
            _style: style,
            _parent: parent,
        };
        self.entries.insert(key, entry);
    }

    /// Drops every entry no element, and no other entry, holds any more.
    pub(crate) fn sweep(&mut self) {
        self.entries
            .retain(|_, entry| Rc::strong_count(&entry.used) > 1);
    }

    /// Replaces every kept `Used` with `forge` of it, for the oracle's
    /// test.
    #[cfg(test)]
    pub(crate) fn forge(&mut self, forge: impl Fn(&mut Used)) {
        for entry in self.entries.values_mut() {
            let mut used = (*entry.used).clone();
            forge(&mut used);
            entry.used = Rc::new(used);
        }
    }
}
