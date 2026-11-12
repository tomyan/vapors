//! The command encoders a [`FrameEncoder`](crate::FrameEncoder) records
//! into: made, finished and submitted, and the flag that says the
//! frame's encoder is still open.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Finishes `encoder` inside a `draw step` span with phase `encode` and
/// submits it inside one with phase `submit`, for vapors-perf.
pub(crate) fn submit_timed(queue: &wgpu::Queue, encoder: wgpu::CommandEncoder) {
    let commands = {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        encoder.finish()
    };
    let _submit = tracing::trace_span!("draw step", phase = "submit").entered();
    queue.submit([commands]);
}

/// A new command encoder labelled `frame`.
pub(crate) fn encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame"),
    })
}

/// Cleared when the encoder is submitted or dropped.
pub(crate) struct Open(pub(crate) Arc<AtomicBool>);

impl Drop for Open {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}
