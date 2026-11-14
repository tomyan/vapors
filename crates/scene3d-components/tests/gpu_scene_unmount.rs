//! GPU tier: unmounting the page's surface drops its `SceneRenderer`,
//! and with it every GPU resource the renderer holds
//! (`docs/plans/3d-2.md` row 5; `RendererLog`, as the racer's
//! `TrackLog`); the app then waits.

mod support;

use support::Rig;
use vapors_app::ElementsApp;
use vapors_gpu_harness::Gpu;
use vapors_platform::{App, Frame, Instant, Next};
use vapors_renderer::clear;
use vapors_scene::Color;

/// One app frame into a fresh 64-pixel target; what it asked for next.
fn frame(app: &mut ElementsApp, gpu: &Gpu) -> Next {
    let texture = clear(&gpu.device, &gpu.queue, 64, 64, Color::default());
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now: Instant::now(),
    })
}

#[test]
fn gpu_unmounting_the_surface_drops_the_scene_renderer_and_the_app_waits() {
    // Given: the page drawn and settled, its renderer holding its GPU
    // resources
    let gpu = vapors_gpu_harness::gpu();
    let mut rig = Rig::new();
    frame(&mut rig.app, &gpu);
    assert_eq!(frame(&mut rig.app, &gpu), Next::Wait);
    assert!(!rig.log.dropped());

    // When: the page leaves the document, then its renderer is
    // unmounted and dropped, as `unmount`'s caller does
    rig.page.unmount(&rig.doc);
    drop(rig.app.unmount("view").expect("mounted"));

    // Then: the removal changed the document, so its frame asks for one
    // more, as any document change does; then the app waits
    assert!(rig.log.dropped(), "unmount kept the renderer alive");
    let after = [frame(&mut rig.app, &gpu), frame(&mut rig.app, &gpu)];
    assert_eq!(after, [Next::Continue, Next::Wait]);
}
