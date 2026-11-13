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

/// The most items in a block: 128 entries of 48 bytes are 6 KB, under
/// a page of 10,000 rows' half (360 KB) by sixty times, and a split
/// copies half of it.
pub(crate) const CAP: usize = 128;

/// A place in the list: the block and the offset within it. The
/// default is the first item's, when there is one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pos {
    block: usize,
    at: usize,
}

/// The list; see the module.
#[derive(Debug)]
pub(crate) struct Blocks<T> {
    blocks: Vec<Vec<T>>,
    len: usize,
    /// Items moved by every insert and remove so far, for the tests
    /// that pin the per-row work.
    moved: usize,
}

impl<T> Blocks<T> {
    /// The list of `items`, already in order.
    pub(crate) fn from_sorted(mut items: Vec<T>) -> Self {
        let len = items.len();
        let mut blocks = Vec::with_capacity(len.div_ceil(CAP));
        while items.len() > CAP {
            let rest = items.split_off(CAP);
            blocks.push(std::mem::replace(&mut items, rest));
        }
        if !items.is_empty() {
            blocks.push(items);
        }
        Self {
            blocks,
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
    #[cfg(test)]
    pub(crate) fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Items moved by every insert and remove so far.
    pub(crate) fn moved(&self) -> usize {
        self.moved
    }

    /// The position after the last item.
    pub(crate) fn end(&self) -> Pos {
        Pos {
            block: self.blocks.len(),
            at: 0,
        }
    }

    pub(crate) fn get(&self, pos: Pos) -> Option<&T> {
        self.blocks.get(pos.block)?.get(pos.at)
    }

    pub(crate) fn get_mut(&mut self, pos: Pos) -> Option<&mut T> {
        self.blocks.get_mut(pos.block)?.get_mut(pos.at)
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
            at: self.blocks[block].len() - 1,
        })
    }

    /// Every item, in order.
    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.blocks.iter().flatten()
    }

    /// The items from `pos` on, in order.
    pub(crate) fn iter_from(&self, pos: Pos) -> impl Iterator<Item = &T> {
        let first = self.blocks.get(pos.block).map(|block| &block[pos.at..]);
        let rest = self.blocks.iter().skip(pos.block + 1);
        first.into_iter().chain(rest.map(Vec::as_slice)).flatten()
    }

    /// The items from `pos` on, in order, to change.
    pub(crate) fn iter_mut_from(&mut self, pos: Pos) -> impl Iterator<Item = &mut T> {
        let mut blocks = self.blocks.iter_mut().skip(pos.block);
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
        let block = self.blocks.partition_point(last_passes);
        let Some(items) = self.blocks.get(block) else {
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

    /// Puts `item` at `pos`, splitting the block first when it is
    /// full; where it is now. Moves at most `CAP` items: a split's
    /// upper half and the tail of the half the item goes in. The block
    /// list's own tail moves too when a block splits: one pointer per
    /// block after it, not counted in `moved`.
    pub(crate) fn insert(&mut self, pos: Pos, item: T) -> Pos {
        self.len += 1;
        if pos.block >= self.blocks.len() {
            return self.push_last(item);
        }
        let pos = if self.blocks[pos.block].len() == CAP {
            self.split(pos)
        } else {
            pos
        };
        let block = &mut self.blocks[pos.block];
        let tail = block.len() - pos.at;
        block.insert(pos.at, item);
        self.moved += tail;
        pos
    }

    /// Splits the full block at `pos` in two halves; `pos` in the half
    /// it falls in.
    fn split(&mut self, pos: Pos) -> Pos {
        let upper = self.blocks[pos.block].split_off(CAP / 2);
        self.moved += upper.len();
        self.blocks.insert(pos.block + 1, upper);
        if pos.at <= CAP / 2 {
            return pos;
        }
        Pos {
            block: pos.block + 1,
            at: pos.at - CAP / 2,
        }
    }

    /// Puts `item` after the last, in the last block when it has room;
    /// where it is.
    fn push_last(&mut self, item: T) -> Pos {
        let last = self.blocks.len().saturating_sub(1);
        match self.blocks.last_mut() {
            Some(block) if block.len() < CAP => {
                block.push(item);
                Pos {
                    block: last,
                    at: block.len() - 1,
                }
            }
            _ => {
                self.blocks.push(vec![item]);
                Pos {
                    block: self.blocks.len() - 1,
                    at: 0,
                }
            }
        }
    }

    /// Takes the item at `pos` out, dropping a block it empties; the
    /// item and the position of the one after it.
    ///
    /// # Panics
    /// When `pos` holds no item.
    pub(crate) fn remove(&mut self, pos: Pos) -> (T, Pos) {
        let block = &mut self.blocks[pos.block];
        let item = block.remove(pos.at);
        let (tail, emptied) = (block.len() - pos.at, block.is_empty());
        self.len -= 1;
        self.moved += tail;
        if emptied {
            self.blocks.remove(pos.block);
        }
        let next = match self.blocks.get(pos.block) {
            Some(block) if pos.at < block.len() => pos,
            Some(_) => Pos {
                block: pos.block + 1,
                at: 0,
            },
            None => self.end(),
        };
        (item, next)
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

#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;
