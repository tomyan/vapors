//! Size queries a lane answers aside (`docs/plans/splice-layout.md`,
//! section 11.2): a lane's starts must stay those its children's
//! layouts were written with, which its lane frames read, because Taffy
//! may serve the container's next layout from its cache and write
//! nothing. A size query with another share or other height bases than
//! the last write, on a lane nothing else changed, is answered from a
//! lane made aside and dropped, O(children). A size query on a lane
//! with changes is answered by the lane itself: its container is laid
//! out again before the update ends, which writes the children and
//! settles the leaves again.

use super::lane::Lane;
use super::lane_fill::again;
use super::lane_kind::LaneKind;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

impl BoxTree<'_> {
    /// Whether this ask of `lane`'s container, as `c`, is a size query
    /// to answer aside: the lane was written, with another share or
    /// other height bases, and nothing in it changed since.
    pub(super) fn answers_aside<K: LaneKind>(&self, c: &K::Container, lane: &Lane<K>) -> bool {
        let Some((share, heights)) = lane.written.as_ref().filter(|_| !K::writes(c)) else {
            return false;
        };
        let rebased = *heights != K::heights(c) || *share != K::share(c);
        rebased && lane.inserted.is_empty() && again(&self.stale, &lane.handles, None).is_empty()
    }

    /// The run of the children of the container in `slot`, as `c`,
    /// from a lane made aside: the container's lane is not touched.
    pub(super) fn fill_aside<K: LaneKind>(
        &mut self,
        slot: usize,
        c: &K::Container,
    ) -> Result<K::Summary, LaneDecline> {
        let mut aside = Lane::<K>::new(K::gap(c));
        aside.share = Some(K::share(c));
        aside.heights = Some(K::heights(c));
        self.fill_all(slot, c, &mut aside)?;
        Ok(aside.tree.total())
    }
}

#[cfg(test)]
#[path = "lane_aside_tests.rs"]
mod tests;
