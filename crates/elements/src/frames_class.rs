//! A frame's class read without composing the frame
//! (`docs/plans/flat-scene.md` 4.3). Paint reads the class of every box
//! it paints again (`repaint.rs`), so a lane frame is not folded for it
//! (the review of #450, M1): each frame takes the class of the frame it
//! is in, so the class is found by walking out to a frame held whole.

use vapors_scene::TransformClass;

use super::Frames;

impl Frames {
    /// The class of frame `frame`. Debug builds check it against the
    /// frame composed afresh.
    ///
    /// # Panics
    /// `frame` is no frame of this paint.
    pub(crate) fn class_of(&self, frame: usize) -> TransformClass {
        let class = self.class_walked(frame);
        #[cfg(debug_assertions)]
        assert_eq!(
            class,
            self.resolved_afresh(frame).class,
            "frame {frame}'s class read without composing it differs from the frame's \
             (frames_class.rs); reproduce with `cargo xtask test`"
        );
        class
    }

    /// A scroll frame's own class, or for a scroll frame kept relative
    /// or a lane frame, the class of the frame it is in.
    fn class_walked(&self, frame: usize) -> TransformClass {
        if let Some(at) = self.by_frame.get(&frame) {
            return at.class;
        }
        if let Some(part) = self.relative.get(&frame) {
            return self.class_walked(part.outer);
        }
        let parent = self.lane_parent(frame).unwrap_or_else(|| {
            panic!(
                "a part names frame {frame}, which this paint does not hold (frames.rs \
                 `Frames::at` says where frames come from); reproduce with `cargo xtask test`"
            )
        });
        self.class_walked(parent)
    }
}
