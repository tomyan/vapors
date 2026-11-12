//! GPU tier: a run whose bytes need a buffer over the device's
//! `max_buffer_size` refuses the frame with `PaintError::Arena`, naming
//! the pipeline, and the painter takes the refusal off the encoder.

#[path = "support/surface.rs"]
mod support;

use support::{Rig, document, update};
use vapors_app::PaintError;
use vapors_gpu_harness::gpu;
use vapors_renderer::ArenaFull;

/// The lowered `max_buffer_size`: room for a 32 px target's readback
/// (32 rows of 256 bytes) and 64 quads of 128 bytes in one draw.
const LIMIT: u64 = 8 * 1024;
/// One more square than one draw of LIMIT bytes holds.
const SQUARES: usize = 65;

#[test]
fn gpu_a_run_over_the_devices_max_buffer_size_refuses_the_frame() {
    // Given: 65 one-pixel red squares, one run of quads
    let css = "body { display: flex; flex-wrap: wrap; } \
               .dot { width: 1px; height: 1px; background-color: #ff0000; }";
    let elements = vec![(None, "div", "dot"); SQUARES];
    let mut doc = document(css, &elements);
    update(&mut doc, 1.0);
    let mut rig = Rig::on(gpu().with_max_buffer_size(LIMIT));

    // When
    let drawn = rig.draw(&doc);

    // Then
    match drawn {
        Err(PaintError::Arena(ArenaFull {
            pipeline, limit, ..
        })) => {
            assert_eq!((pipeline, limit), ("quads", LIMIT));
        }
        other => panic!(
            "expected PaintError::Arena, got {:?}",
            other.map(|_| "pixels")
        ),
    }
}
