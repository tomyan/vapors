//! Every batch's rows for one view's batches in one table
//! (`framed_batches.rs`, 4f): a lane batch's row frames and each item's
//! row, side by side with the other batches', so making a view's batches
//! allocates a few vectors, not two a batch.

/// The rows of each batch of a view, by the batch's place: a batch in one
/// frame has none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowsOfBatches {
    frames: Vec<usize>,
    items: Vec<u32>,
    /// By batch: where its row frames and its items' rows start in
    /// `frames` and `items`; each runs to the next batch's start.
    starts: Vec<(usize, usize)>,
}

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
        self.starts.len()
    }

    /// Whether it holds no batch.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }

    /// The rows of the batch at `at`, if there is one.
    #[must_use]
    pub fn get(&self, at: usize) -> Option<BatchRows<'_>> {
        let &(frames, items) = self.starts.get(at)?;
        let (frames_end, items_end) = self
            .starts
            .get(at + 1)
            .copied()
            .unwrap_or((self.frames.len(), self.items.len()));
        Some(BatchRows {
            frames: &self.frames[frames..frames_end],
            items: &self.items[items..items_end],
        })
    }

    /// Each batch's rows, in order.
    pub fn iter(&self) -> impl Iterator<Item = BatchRows<'_>> {
        (0..self.len()).filter_map(|at| self.get(at))
    }

    /// A batch with no rows yet added after the others.
    pub(crate) fn start_batch(&mut self) {
        self.starts.push((self.frames.len(), self.items.len()));
    }

    /// `count` batches in one frame, with no rows, added.
    pub(crate) fn add_unrowed(&mut self, count: usize) {
        let at = (self.frames.len(), self.items.len());
        self.starts.extend(std::iter::repeat_n(at, count));
    }

    /// An item in row frame `row` added to the last batch: its rows are
    /// numbered as they first appear in it. A row's items are mostly
    /// consecutive, so the last row is tried first, then the batch's rows
    /// (at most `vapors_scene::ROWS_PER_BATCH`) in turn.
    ///
    /// # Panics
    ///
    /// When no batch was started: an item belongs to one.
    pub(crate) fn push_item(&mut self, row: usize) {
        let first = self.starts.last().expect("a batch for the item").0;
        let rows = &self.frames[first..];
        let found = (rows.last() == Some(&row))
            .then(|| rows.len() - 1)
            .or_else(|| rows.iter().rposition(|&frame| frame == row));
        let at = found.unwrap_or_else(|| {
            self.frames.push(row);
            self.frames.len() - 1 - first
        });
        self.items.push(u32::try_from(at).expect("under 2^32 rows"));
    }

    /// `other`'s batches added after these.
    pub(crate) fn append(&mut self, other: &RowsOfBatches) {
        let moved = (self.frames.len(), self.items.len());
        let starts = other
            .starts
            .iter()
            .map(|&(f, i)| (f + moved.0, i + moved.1));
        self.starts.extend(starts);
        self.frames.extend_from_slice(&other.frames);
        self.items.extend_from_slice(&other.items);
    }

    /// One batch's rows made from `frames` and `items`, for the tests.
    #[cfg(test)]
    pub(crate) fn one(frames: &[usize], items: &[u32]) -> Self {
        Self {
            frames: frames.to_vec(),
            items: items.to_vec(),
            starts: vec![(0, 0)],
        }
    }
}

#[cfg(test)]
#[path = "framed_batches_arena_tests.rs"]
mod tests;
