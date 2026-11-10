//! Root module so the test ids start with `gpu_` (the tier split).

use vapors_gpu_harness::gpu;
use vapors_platform::Instant;

use crate::ScenePainter;
use crate::elements_app::give_one_loop_waker_to;
use crate::painter::{SCOPE_DEADLINE, wants_frame_around_a_pending_scope};

#[test]
fn gpu_an_idle_backed_surface_with_a_pending_scope_wants_no_frame_until_its_deadline() {
    // Given
    let now = Instant::now();

    // When
    let [before, after] = wants_frame_around_a_pending_scope(now);

    // Then
    assert_eq!(before, (false, None));
    assert_eq!(after, (false, Some(now + SCOPE_DEADLINE)));
}

#[test]
fn gpu_a_painter_made_again_is_given_the_loops_waker_too() {
    // Given
    let gpu = gpu();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let made = || ScenePainter::new(&gpu.device, format).expect("a supported format");
    let (mut first, mut again) = (made(), made());

    // When
    give_one_loop_waker_to([&mut first, &mut again]);

    // Then
    assert!(first.has_waker());
    assert!(again.has_waker());
}
