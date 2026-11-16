//! Every batch's rows for one view's batches in one table
//! (`framed_batches.rs`, 4f): a lane batch's row frames and each item's
//! row, side by side with the other batches', so making a view's batches
//! allocates a few vectors, not two a batch.

/// The rows of each batch of a view, by the batch's place: a batch in one
/// frame has none. The batches of one lane part share its table, which
/// gives the lane frame in each slot (`NO_ROW` for a slot no row holds).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowsOfBatches {
    frames: Vec<usize>,
    items: Vec<u32>,
    /// By part: its table's place in `frames`.
    parts: Vec<(usize, usize)>,
    /// By batch: its part, if it is a lane batch, and where its items'
    /// slots start in `items`; each runs to the next batch's start.
    batches: Vec<(Option<usize>, usize)>,
}

/// A table entry for a slot no row holds.
pub(crate) const NO_ROW: usize = usize::MAX;

/// The rows a batch over a lane's rows draws: `frames[i]` is the lane
/// frame row `i` names, `items[j]` item `j`'s row. Both empty for a batch
/// in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BatchRows<'a> {
    pub(crate) frames: &'a [usize],
    pub(crate) items: &'a [u32],
}

impl BatchRows<'_> {
    /// Each item's row, by its place in the batch's run; empty for a
    /// batch in one frame.
    #[must_use]
    pub fn items(&self) -> &[u32] {
        self.items
    }
}

impl RowsOfBatches {
    /// How many batches it holds rows for.
    #[must_use]
    pub fn len(&self) -> usize {
        self.batches.len()
    }

    /// Whether it holds no batch.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.batches.is_empty()
    }

    /// The rows of the batch at `at`, if there is one.
    #[must_use]
    pub fn get(&self, at: usize) -> Option<BatchRows<'_>> {
        let &(part, items) = self.batches.get(at)?;
        let items_end = self
            .batches
            .get(at + 1)
            .map_or(self.items.len(), |next| next.1);
        let frames = part.map_or(&[][..], |part| {
            let (start, end) = self.parts[part];
            &self.frames[start..end]
        });
        Some(BatchRows {
            frames,
            items: &self.items[items..items_end],
        })
    }

    /// The lane part of the batch at `at`, whose table its rows share with
    /// the part's other batches; `None` for a batch in one frame.
    #[must_use]
    pub fn part(&self, at: usize) -> Option<usize> {
        self.batches.get(at).and_then(|&(part, _)| part)
    }

    /// Each batch's rows, in order.
    pub fn iter(&self) -> impl Iterator<Item = BatchRows<'_>> {
        (0..self.len()).filter_map(|at| self.get(at))
    }

    /// A lane part opened, its table to come (`close_part`): its index.
    pub(crate) fn open_part(&mut self) -> usize {
        self.parts.push((self.frames.len(), self.frames.len()));
        self.parts.len() - 1
    }

    /// Part `part`'s table: the lane frame in each slot.
    pub(crate) fn close_part(&mut self, part: usize, table: &[usize]) {
        let start = self.frames.len();
        self.frames.extend_from_slice(table);
        self.parts[part] = (start, self.frames.len());
    }

    /// A batch of part `part` with no items yet added after the others.
    pub(crate) fn start_batch(&mut self, part: usize) {
        self.batches.push((Some(part), self.items.len()));
    }

    /// `count` batches in one frame, with no rows, added.
    pub(crate) fn add_unrowed(&mut self, count: usize) {
        let at = (None, self.items.len());
        self.batches.extend(std::iter::repeat_n(at, count));
    }

    /// An item in the slot `entry` of its part's table added to the last
    /// batch.
    pub(crate) fn push_item(&mut self, entry: u32) {
        self.items.push(entry);
    }

    /// `other`'s batches added after these.
    pub(crate) fn append(&mut self, other: &RowsOfBatches) {
        let (frames, items, parts) = (self.frames.len(), self.items.len(), self.parts.len());
        let moved = other.parts.iter().map(|&(s, e)| (s + frames, e + frames));
        self.parts.extend(moved);
        let batches = other
            .batches
            .iter()
            .map(|&(part, at)| (part.map(|p| p + parts), at + items));
        self.batches.extend(batches);
        self.frames.extend_from_slice(&other.frames);
        self.items.extend_from_slice(&other.items);
    }

    /// One batch's rows made from its part's `table` and its items'
    /// `entries`, for the tests.
    #[cfg(test)]
    pub(crate) fn one(table: &[usize], entries: &[u32]) -> Self {
        let mut rows = Self::default();
        let part = rows.open_part();
        rows.close_part(part, table);
        rows.start_batch(part);
        for &entry in entries {
            rows.push_item(entry);
        }
        rows
    }
}

#[cfg(test)]
#[path = "framed_batches_arena_tests.rs"]
mod tests;
