//! Lists of one entry per box slot (or per node), resized to a new
//! count, grown by a quarter rather than doubled. They are kept between
//! updates and sized exactly by the first full build, so the first box
//! an edit adds would double each one: at 16,384 rows the kept boxes'
//! list (about 740 bytes a slot) went from 48.6 MB to 97.3 MB for one
//! added row (property 8, docs/perf/memory-per-element-2026-11-14.md).
//! A quarter bounds the slack at 25% and keeps one copy per growth
//! amortised over a quarter of the list's length in pushes.

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

#[cfg(test)]
#[path = "slot_list_tests.rs"]
mod tests;
