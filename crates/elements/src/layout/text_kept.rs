//! A map of values kept between updates, each with the update that last
//! asked for it, swept by age (`text_cache.rs` gives the bound and its
//! cost). A value something outside the map holds is never swept.

use std::hash::Hash;
use std::rc::Rc;

use rustc_hash::FxHashMap;

/// Whether something outside the map holds a value, so a sweep keeps it.
pub(crate) trait Held {
    fn held(&self) -> bool;
}

impl<T> Held for Rc<T> {
    /// A kept paint holds the layouts it shows.
    fn held(&self) -> bool {
        Rc::strong_count(self) > 1
    }
}

/// Values by key, each with the update that last asked for it.
#[derive(Clone)]
pub(crate) struct Kept<K, V> {
    pub(super) entries: FxHashMap<K, Entry<V>>,
    /// The update under way, counting from 1 (`begin`).
    update: u64,
    /// How many entries the last sweep left.
    pub(super) swept_to: usize,
    /// How many entries this update asked for or inserted.
    asked_now: usize,
}

#[derive(Clone)]
pub(crate) struct Entry<V> {
    pub(super) value: V,
    pub(super) asked: u64,
}

impl<K, V> Default for Kept<K, V> {
    fn default() -> Self {
        Self {
            entries: FxHashMap::default(),
            update: 0,
            swept_to: 0,
            asked_now: 0,
        }
    }
}

impl<K: Hash + Eq, V: Held> Kept<K, V> {
    /// The value kept by `key`, recorded as asked for this update.
    pub(crate) fn asked(&mut self, key: &K) -> Option<&V> {
        let entry = self.entries.get_mut(key)?;
        if entry.asked != self.update {
            entry.asked = self.update;
            self.asked_now += 1;
        }
        Some(&entry.value)
    }

    /// Keeps `value` by `key`, sweeping first when the map has doubled
    /// since the last sweep.
    pub(crate) fn insert(&mut self, key: K, value: V) {
        if self.entries.len() >= 2 * self.swept_to.max(1) {
            self.sweep();
        }
        let asked = self.update;
        self.entries.insert(key, Entry { value, asked });
        self.asked_now += 1;
    }

    /// Drops the entries asked for before the last update that nothing
    /// outside the map holds.
    fn sweep(&mut self) {
        let since = self.update.saturating_sub(1);
        self.entries
            .retain(|_, entry| entry.asked >= since || entry.value.held());
        self.swept_to = self.entries.len();
    }

    /// Starts an update, the last one's paint showing `shown` values;
    /// sweeps when the map holds more than 4 times the live set.
    pub(crate) fn begin(&mut self, shown: usize) {
        let live = self.asked_now.max(shown).max(1);
        self.update += 1;
        self.asked_now = 0;
        if self.entries.len() > 4 * live {
            self.sweep();
        }
    }
}
