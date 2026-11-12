//! Which encoder a pipeline's buffer last took a draw for, and the
//! checks that keep one encoder's bytes from overwriting another's:
//! shared by the per-frame arenas and the kept uniform slots.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::FrameEncoder;

/// The encoder drawn for last, and whether it is still open. A newer
/// encoder may begin once the older is submitted or dropped; an older
/// one never.
pub(crate) struct EncoderTurn {
    label: &'static str,
    encoder: u64,
    open: Arc<AtomicBool>,
}

impl EncoderTurn {
    pub(crate) fn new(label: &'static str) -> Self {
        Self {
            label,
            encoder: 0,
            open: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Takes a draw for `frame`: true when it is a newer encoder than
    /// the last one drawn for, which is then closed.
    ///
    /// # Panics
    ///
    /// If `frame` is older than the last encoder, or newer while that
    /// encoder is still open (neither submitted nor dropped): either
    /// way one encoder's bytes would overwrite the other's.
    pub(crate) fn begin(&mut self, frame: &FrameEncoder) -> bool {
        assert!(
            frame.id() >= self.encoder,
            "{}: a draw for encoder {} after encoder {}, an older encoder after a newer one; \
             submit each FrameEncoder before drawing with the next",
            self.label,
            frame.id(),
            self.encoder
        );
        if frame.id() == self.encoder {
            return false;
        }
        assert!(
            !self.open.load(Ordering::Relaxed),
            "{}: a draw for encoder {} while encoder {}, which this pipeline drew for, is still open; \
             submit or drop each FrameEncoder before drawing with the next",
            self.label,
            frame.id(),
            self.encoder
        );
        (self.encoder, self.open) = (frame.id(), frame.open_flag());
        true
    }
}
