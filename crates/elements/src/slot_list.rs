//! Lists of one entry per box slot (or per node), resized to a new
//! count, grown by a quarter rather than doubled. They are kept between
//! updates and sized exactly by the first full build, so the first box
//! an edit adds would double each one: at 16,384 rows the kept boxes'
//! list (about 740 bytes a slot) went from 48.6 MB to 97.3 MB for one
//! added row (property 8, docs/perf/memory-per-element-2026-11-14.md).
//! While a list grows its slack stays within a quarter of its length,
//! and each growth's copy is amortised over a quarter of its length in
//! pushes. A shorter length keeps the capacity: freed slots are taken
//! again by the next boxes (the free list), so a list never shrinks.
//!
//! A kept per-slot list is a [`SlotList`], whose only growth goes
//! through here, so no list can double behind it; and each owner names
//! its lists in one `slot_lists`, which the growth test reads
//! (`slot_list_inventory_tests.rs` fails when a field is left out).

/// The capacity a list of `len` entries needs to grow to for `wanted`:
/// unchanged when it holds them, else the larger of `wanted` and a
/// quarter more than it held.
fn grown(capacity: usize, wanted: usize) -> usize {
    if wanted <= capacity {
        return capacity;
    }
    wanted.max(capacity + capacity / 4)
}

/// Makes room in `list` for `wanted` entries, by a quarter at least.
pub(crate) fn reserve<T>(list: &mut Vec<T>, wanted: usize) {
    let target = grown(list.capacity(), wanted);
    list.reserve_exact(target - list.len());
}

/// `list.resize(len, value)`, grown by a quarter.
pub(crate) fn resize<T: Clone>(list: &mut Vec<T>, len: usize, value: T) {
    reserve(list, len);
    list.resize(len, value);
}

/// `list.resize_with(len, make)`, grown by a quarter.
pub(crate) fn resize_with<T>(list: &mut Vec<T>, len: usize, make: impl FnMut() -> T) {
    reserve(list, len);
    list.resize_with(len, make);
}

/// A kept list of one entry per slot: a `Vec` read as a slice, grown
/// only by a quarter (`resize`, `resize_with`, `push`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SlotList<T>(Vec<T>);

impl<T> Default for SlotList<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> std::ops::Deref for SlotList<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<T> std::ops::DerefMut for SlotList<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.0
    }
}

/// A list made whole (a first build), its capacity cut to its length.
impl<T> From<Vec<T>> for SlotList<T> {
    fn from(mut list: Vec<T>) -> Self {
        list.shrink_to_fit();
        Self(list)
    }
}

/// A list made whole (a first build), its capacity cut to its length.
impl<T> FromIterator<T> for SlotList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut list: Vec<T> = items.into_iter().collect();
        list.shrink_to_fit();
        Self(list)
    }
}

impl<T> SlotList<T> {
    /// `len` entries, the new ones `value`, grown by a quarter.
    pub(crate) fn resize(&mut self, len: usize, value: T)
    where
        T: Clone,
    {
        resize(&mut self.0, len, value);
    }

    /// `len` entries, the new ones made by `make`, grown by a quarter.
    pub(crate) fn resize_with(&mut self, len: usize, make: impl FnMut() -> T) {
        resize_with(&mut self.0, len, make);
    }

    /// `item` added at the end, grown by a quarter.
    pub(crate) fn push(&mut self, item: T) {
        let wanted = self.0.len() + 1;
        reserve(&mut self.0, wanted);
        self.0.push(item);
    }

    /// The entries from `start` on, taken out; the capacity is kept.
    pub(crate) fn drain_from(&mut self, start: usize) -> std::vec::Drain<'_, T> {
        self.0.drain(start..)
    }

    /// Its name, length and capacity, for an owner's `slot_lists`.
    #[cfg(test)]
    pub(crate) fn named(&self, name: &'static str) -> (&'static str, usize, usize) {
        (name, self.0.len(), self.0.capacity())
    }
}

#[cfg(test)]
#[path = "slot_list_tests.rs"]
mod tests;
