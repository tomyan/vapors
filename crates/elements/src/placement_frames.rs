//! Lane frames in placement (`docs/plans/splice-layout.md`, section
//! 11.2): when the walk places a lane container it decides whether the
//! lane's children are frames of their own (`lane_frame.rs`); a framed
//! child's rect leaves the lane's start out, so an insert moves no later
//! child, and a container moved only along its axis moves none of its
//! framed children either: their frames' shifts carry it.

use super::Walk;

impl Walk<'_, '_> {
    /// Decides whether the children of `slot`, its content at `inner`,
    /// are frames, noting a decline; whether that changed, which places
    /// every child again.
    pub(crate) fn frame_lane(&mut self, slot: usize, inner: (f32, f32)) -> bool {
        let lanes = self.boxes.lanes;
        let decision = lanes.framing(slot, f64::from(inner.1), self.boxes.scale);
        if let Some(Err(decline)) = decision {
            self.placed.unframed.push((slot, decline));
        }
        let was = match decision {
            Some(decision) => self.kept.framing.insert(slot, decision),
            None => self.kept.framing.remove(&slot),
        };
        was != decision
    }

    /// Whether the children of `slot` are placed again because its
    /// content moved from `before` to `inner`: not when it moved only
    /// along the axis of a lane whose children are frames.
    // Exact, as the whole content origin is compared otherwise.
    #[allow(clippy::float_cmp)]
    pub(crate) fn children_moved(
        &self,
        slot: usize,
        inner: (f32, f32),
        before: (f32, f32),
    ) -> bool {
        let framed = matches!(self.kept.framing.get(&slot), Some(Ok(())));
        if framed {
            inner.0 != before.0
        } else {
            inner != before
        }
    }

    /// Whether `slot` is a child of a lane whose children are frames.
    pub(crate) fn lane_framed(&self, slot: usize) -> bool {
        let up = self.kept.up[slot];
        let framed = matches!(self.kept.framing.get(&up), Some(Ok(())));
        framed && self.boxes.lanes.frame_offset(up, slot).is_some()
    }
}
