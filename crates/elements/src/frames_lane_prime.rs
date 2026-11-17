//! The started rows' lane frames resolved in one pass (`frames_lane.rs`;
//! `docs/plans/splice-layout.md`, section 11.2, 4d).

use super::super::{FrameAt, Frames, meet};
use super::LaneBase;
use crate::paint::snapped;

/// A primed table entry for a slot not primed: no template has it.
const NOT_PRIMED: (u32, [f32; 2]) = (u32::MAX, [0.0; 2]);

/// The frames `prime_started` worked out: each primed lane's template
/// (`lane_template`), and by slot the template's index and the child's
/// shift (`NOT_PRIMED` for none).
#[derive(Debug, Default)]
pub(super) struct Primed {
    pub(super) templates: Vec<FrameAt>,
    pub(super) table: Vec<(u32, [f32; 2])>,
}

impl Frames {
    /// Lane frame `frame`'s shift when `prime_started` worked it out.
    pub(in crate::frames) fn primed_shift(&self, frame: usize) -> Option<[f32; 2]> {
        let primed = self.lanes.primed.get()?;
        let &(template, shift) = primed.table.get(frame)?;
        (template != NOT_PRIMED.0).then_some(shift)
    }

    /// Lane frame `frame` when `prime_started` worked it out.
    pub(in crate::frames) fn primed_at(&self, frame: usize) -> Option<FrameAt> {
        let primed = self.lanes.primed.get()?;
        let &(template, shift) = primed.table.get(frame)?;
        let template = primed.templates.get(template as usize)?;
        Some(FrameAt { shift, ..*template })
    }

    /// What every child of the lane at `base` shares, its container's
    /// content in the frame at `outer`: the clip, and the outer shift
    /// before the child's own offset (`lane_child_at`).
    pub(super) fn lane_template(&self, base: LaneBase, outer: FrameAt) -> FrameAt {
        let clip = match base.inner {
            Some(inner) => meet(Some(self.css_rect(outer, inner)), outer.clip),
            None => outer.clip,
        };
        FrameAt {
            shift: outer.shift,
            clip,
            device_clip: clip.map(|clip| snapped(clip, self.scale)),
            class: outer.class,
        }
    }

    /// The frame of the child in `slot`, `offset` down the lane at
    /// `base`, from the lane's `template`.
    #[allow(clippy::cast_possible_truncation)] // device px, whole, within f32
    pub(super) fn lane_child_at(
        &self,
        template: FrameAt,
        base: LaneBase,
        (slot, offset): (usize, f64),
    ) -> FrameAt {
        let down = (f64::from(base.origin) + offset) * f64::from(self.scale);
        let shift = [template.shift[0], template.shift[1] + down as f32];
        #[cfg(debug_assertions)]
        super::super::assert_whole_shift(slot, template.class, shift);
        let _ = slot;
        FrameAt { shift, ..template }
    }

    /// Resolves the frames of this update's started children, one pass
    /// along each framed lane (sequential starts, `Lanes::each_offset_from`),
    /// into the memo, once per update: the a11y take reads each started
    /// child's bounds, one fold of the lane each otherwise (section
    /// 11.2, 4d). A framed lane's starts are whole device pixels, so
    /// they equal the fold's. Rows started by an earlier update with no
    /// take since resolve by fold when read.
    pub(crate) fn prime_started(&self) {
        if self.lanes.primed.get().is_some() {
            return;
        }
        let mut primed = Primed::default();
        for &started in &self.lanes.started {
            if let Some(&base) = self.lanes.bases.get(&started.0) {
                self.prime_lane(base, started, &mut primed);
            }
        }
        _ = self.lanes.primed.set(primed);
        if cfg!(debug_assertions) {
            for &started in &self.lanes.started {
                if let Some(&base) = self.lanes.bases.get(&started.0) {
                    self.assert_lane_primed(base, started);
                }
            }
        }
    }

    /// Works out the frames of the children of the framed lane of
    /// `container`, at `base`, from the one in `first` on, in one pass,
    /// into `primed`. Frames are read meanwhile (a lane nested in a
    /// framed row reads its outer lane frame, by fold until `primed` is
    /// set).
    fn prime_lane(&self, base: LaneBase, started: (usize, usize), primed: &mut Primed) {
        let shared = self
            .lanes
            .shared
            .as_ref()
            .expect("lane bases come with their lanes");
        let template = self.lane_template(base, self.at(base.parent));
        let index = u32::try_from(primed.templates.len()).expect("under 2^32 lanes");
        primed.templates.push(template);
        let table = &mut primed.table;
        shared
            .borrow()
            .each_offset_from(started, &mut |slot, offset| {
                if table.len() <= slot {
                    table.resize(slot + 1, NOT_PRIMED);
                }
                table[slot] = (
                    index,
                    self.lane_child_at(template, base, (slot, offset)).shift,
                );
            });
    }

    /// Panics, naming the frame, when a frame primed in the lane of
    /// `container` at `base` from `first` on is not the one the lane's
    /// fold gives.
    fn assert_lane_primed(&self, base: LaneBase, (container, first): (usize, usize)) {
        let shared = self
            .lanes
            .shared
            .as_ref()
            .expect("lane bases come with their lanes");
        let outer = self.at(base.parent);
        let mut slots = Vec::new();
        shared
            .borrow()
            .each_offset_from((container, first), &mut |slot, _| slots.push(slot));
        for slot in slots {
            let at = self.primed_at(slot).expect("primed");
            self.assert_primed_folds((container, slot), (base, outer), at);
        }
    }

    /// Panics, naming the frame, when `primed`, worked out in one pass,
    /// is not the frame the lane's fold for `slot` gives.
    fn assert_primed_folds(
        &self,
        (container, slot): (usize, usize),
        (base, outer): (LaneBase, FrameAt),
        primed: FrameAt,
    ) {
        let shared = self
            .lanes
            .shared
            .as_ref()
            .expect("lane bases come with their lanes");
        let folded = shared.borrow().frame_offset(container, slot);
        let folded = folded.map(|offset| self.lane_frame_at(base, (slot, offset), outer));
        assert_eq!(
            Some(primed),
            folded,
            "lane frame {slot} primed in one pass along its lane (frames_lane.rs \
             `prime_started`) is not the frame its lane's fold gives; reproduce with \
             `cargo xtask test`"
        );
    }
}

#[cfg(test)]
#[path = "frames_lane_prime_tests.rs"]
mod tests;
