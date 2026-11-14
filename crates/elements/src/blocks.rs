//! An ordered list kept in blocks of at most [`CAP`] items, so that
//! putting an item in or taking one out moves at most a block's tail,
//! never the list's (a `Vec`'s insert in the middle of 10,000 rows'
//! entries moved half of them, `docs/perf/class-add-paint-2026-11-13.md`).
//! The drawing steps (`drawn_steps.rs`) and each scroll frame's extents
//! index (`frame_extents.rs`) are kept in one. Searches are binary over
//! the blocks' last items, then within the block. A block that fills
//! splits in two; one that empties is dropped; blocks are never merged,
//! so a block count up to the item count keeps every search
//! logarithmic and every move under `CAP`. A position is a block and an
//! offset, valid until the next insert or remove; the end is the
//! position after the last block. The layout lane's sum tree
//! (`docs/plans/splice-layout.md`) may converge with this type later.

/// The most items in a block: 128 entries of 48 bytes are 6 KB,
/// against 240 KB for half of 10,000 rows' entries, and a split copies
/// half of it.
pub(crate) const CAP: usize = 128;

/// A place in the list: the block and the offset within it. The
/// default is the first item's, when there is one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pos {
    block: usize,
    at: usize,
}

/// What an update's patches did to the two indexes kept in blocks,
/// for `UpdateCounts`: entries moved (block pointers counted as one),
/// times the extents index was made afresh inside a patch, and the
/// blocks both hold after it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IndexCounts {
    pub(crate) moved: usize,
    pub(crate) rebuilds: usize,
    pub(crate) blocks: usize,
}

impl IndexCounts {
    /// Both indexes' counts together.
    pub(crate) fn and(self, other: Self) -> Self {
        Self {
            moved: self.moved + other.moved,
            rebuilds: self.rebuilds + other.rebuilds,
            blocks: self.blocks + other.blocks,
        }
    }
}

/// The list; see the module.
#[derive(Debug)]
pub(crate) struct Blocks<T> {
    chunks: Vec<Vec<T>>,
    len: usize,
    /// Items moved by every insert and remove so far, a block pointer
    /// moved in the block list counting as one, for the tests that pin
    /// the per-row work.
    moved: usize,
}

impl<T> Blocks<T> {
    /// The list of `items`, already in order.
    pub(crate) fn from_sorted(mut items: Vec<T>) -> Self {
        let len = items.len();
        let mut chunks = Vec::with_capacity(len.div_ceil(CAP));
        while items.len() > CAP {
            let rest = items.split_off(CAP);
            chunks.push(std::mem::replace(&mut items, rest));
        }
        if !items.is_empty() {
            chunks.push(items);
        }
        Self {
            chunks,
            len,
            moved: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// How many blocks hold the items.
    pub(crate) fn block_count(&self) -> usize {
        self.chunks.len()
    }

    /// Items moved by every insert and remove so far.
    #[cfg(test)]
    pub(crate) fn moved(&self) -> usize {
        self.moved
    }

    /// Items moved by every insert and remove since the last take.
    pub(crate) fn take_moved(&mut self) -> usize {
        std::mem::take(&mut self.moved)
    }

    /// The position after the last item.
    pub(crate) fn end(&self) -> Pos {
        Pos {
            block: self.chunks.len(),
            at: 0,
        }
    }

    pub(crate) fn get(&self, pos: Pos) -> Option<&T> {
        self.chunks.get(pos.block)?.get(pos.at)
    }

    pub(crate) fn get_mut(&mut self, pos: Pos) -> Option<&mut T> {
        self.chunks.get_mut(pos.block)?.get_mut(pos.at)
    }

    /// The position before `pos`; `None` at the first item.
    pub(crate) fn prev(&self, pos: Pos) -> Option<Pos> {
        if pos.at > 0 {
            return Some(Pos {
                block: pos.block,
                at: pos.at - 1,
            });
        }
        let block = pos.block.checked_sub(1)?;
        Some(Pos {
            block,
            at: self.chunks[block].len() - 1,
        })
    }

    /// Every item, in order.
    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.chunks.iter().flatten()
    }

    /// The items from `pos` on, in order.
    pub(crate) fn iter_from(&self, pos: Pos) -> impl Iterator<Item = &T> {
        let first = self.chunks.get(pos.block).map(|block| &block[pos.at..]);
        let rest = self.chunks.iter().skip(pos.block + 1);
        first.into_iter().chain(rest.map(Vec::as_slice)).flatten()
    }

    /// The items from `pos` on, in order, to change.
    pub(crate) fn iter_mut_from(&mut self, pos: Pos) -> impl Iterator<Item = &mut T> {
        let mut blocks = self.chunks.iter_mut().skip(pos.block);
        let first = blocks.next().map(|block| &mut block[pos.at..]);
        first
            .into_iter()
            .chain(blocks.map(Vec::as_mut_slice))
            .flatten()
    }

    /// The first position whose item fails `pred`, which every item
    /// before it passes; the end when all pass.
    pub(crate) fn partition_point(&self, mut pred: impl FnMut(&T) -> bool) -> Pos {
        let last_passes = |block: &Vec<T>| pred(block.last().expect("blocks are never empty"));
        let block = self.chunks.partition_point(last_passes);
        let Some(items) = self.chunks.get(block) else {
            return self.end();
        };
        Pos {
            block,
            at: items.partition_point(pred),
        }
    }

    /// The position of the item `f` finds equal, or the position to
    /// insert one at.
    pub(crate) fn binary_search_by(
        &self,
        mut f: impl FnMut(&T) -> std::cmp::Ordering,
    ) -> Result<Pos, Pos> {
        let pos = self.partition_point(|item| f(item).is_lt());
        match self.get(pos) {
            Some(item) if f(item).is_eq() => Ok(pos),
            _ => Err(pos),
        }
    }
}

impl<T> Default for Blocks<T> {
    fn default() -> Self {
        Self::from_sorted(Vec::new())
    }
}

impl<T: PartialEq> PartialEq for Blocks<T> {
    /// The same items in the same order, however blocked.
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}

#[path = "blocks_edit.rs"]
mod edit;

#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "blocks_edit_tests.rs"]
mod edit_tests;
