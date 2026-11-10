//! GPU tier: the order of unmounting the track's renderer and removing
//! its `gpu-surface` element (docs/gpu-surface.md, 3D slice 1c PR 6).
//! Resources are released by Rust ownership: `log.dropped` shows the
//! renderer was dropped in each order.
use crate::gpu_track_unmount_tests::animated;
use vapors_gpu_harness::gpu;
use vapors_platform::Next;

#[test]
fn gpu_unmounting_before_removing_the_element_leaves_a_page_that_waits() {
    // Given
    let gpu = gpu();
    let (mut page, _handle, log) = animated(&gpu);

    // When: the renderer goes first, then the element
    page.unmount();
    page.remove_element();
    let after = [page.frame(&gpu), page.frame(&gpu)];

    // Then
    assert!(log.dropped.get(), "unmount kept the renderer alive");
    assert_eq!(after, [Next::Continue, Next::Wait]);
    assert_eq!(log.renders.get(), 2);
}

#[test]
fn gpu_unmounting_without_removing_the_element_fails_the_next_frame_naming_the_fix() {
    // Given
    let gpu = gpu();
    let (mut page, _handle, log) = animated(&gpu);

    // When: the element stays, painted with no renderer
    page.unmount();
    page.frame_allowing_failure(&gpu);

    // Then: the frame fails, naming the fix; it does not draw stale content
    assert!(log.dropped.get(), "unmount kept the renderer alive");
    let failure = page.app.failure().get().expect("the frame failed");
    assert!(
        failure.contains(
            "the gpu-surface \"track\" has no renderer; mount one with ScenePainter::mount"
        ),
        "{failure}"
    );
    assert_eq!(log.renders.get(), 2);
}
