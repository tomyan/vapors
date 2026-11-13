//! Putting an item into a blocked list and taking one out
//! (`blocks.rs`): a full block splits first, an emptied block is
//! dropped, and each operation moves at most `CAP` items.

use super::{Blocks, CAP, Pos};

impl<T> Blocks<T> {
    /// Puts `item` at `pos`, splitting the block first when it is
    /// full; where it is now. Moves at most `CAP` items, a split's
    /// upper half and the tail of the half the item goes in, plus the
    /// block list's own tail when a block splits, one pointer per block
    /// after it; all counted in `moved`.
    pub(crate) fn insert(&mut self, pos: Pos, item: T) -> Pos {
        self.len += 1;
        if pos.block >= self.chunks.len() {
            return self.push_last(item);
        }
        let pos = if self.chunks[pos.block].len() == CAP {
            self.split(pos)
        } else {
            pos
        };
        let block = &mut self.chunks[pos.block];
        let tail = block.len() - pos.at;
        block.insert(pos.at, item);
        self.moved += tail;
        pos
    }

    /// Splits the full block at `pos` in two halves; `pos` in the half
    /// it falls in.
    fn split(&mut self, pos: Pos) -> Pos {
        let upper = self.chunks[pos.block].split_off(CAP / 2);
        self.moved += upper.len() + (self.chunks.len() - pos.block - 1);
        self.chunks.insert(pos.block + 1, upper);
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
        if self.chunks.last().is_none_or(|last| last.len() == CAP) {
            self.chunks.push(Vec::with_capacity(CAP / 2));
        }
        let block = self.chunks.len() - 1;
        let last = &mut self.chunks[block];
        last.push(item);
        Pos {
            block,
            at: last.len() - 1,
        }
    }

    /// Takes the item at `pos` out, dropping a block it empties (which
    /// moves the block pointers after it); the item and the position
    /// of the one after it.
    ///
    /// # Panics
    /// When `pos` holds no item.
    pub(crate) fn remove(&mut self, pos: Pos) -> (T, Pos) {
        let block = &mut self.chunks[pos.block];
        let item = block.remove(pos.at);
        let (tail, emptied) = (block.len() - pos.at, block.is_empty());
        self.len -= 1;
        self.moved += tail;
        if emptied {
            self.chunks.remove(pos.block);
            self.moved += self.chunks.len() - pos.block;
        }
        (item, self.after_removal(pos))
    }

    /// The position of the item after one just removed at `pos`.
    fn after_removal(&self, pos: Pos) -> Pos {
        match self.chunks.get(pos.block) {
            Some(block) if pos.at < block.len() => pos,
            Some(_) => Pos {
                block: pos.block + 1,
                at: 0,
            },
            None => self.end(),
        }
    }
}
