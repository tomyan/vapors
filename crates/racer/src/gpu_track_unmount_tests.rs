//! GPU tier: the track's renderer unmounted while it animates, on a
//! test page that holds only its `gpu-surface` (the racer's HUD always
//! has one). The surface element is removed and the renderer unmounted:
//! dropping what `unmount` hands back drops the renderer and the GPU
//! resources it owns; its handle refuses every call as `NotMounted`; the
//! app waits; a new track mounted on a new surface renders
//! (docs/gpu-surface.md, 3D slice 1c PR 6).
use crate::gpu::{Scene, Track, TrackLog};
use crate::gpu_frames::MAGENTA;
use std::{cell::RefCell, rc::Rc};
use vapors_app::{ElementsApp, InvalidateError, SurfaceHandle};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Frame, Instant, Next};
use vapors_renderer::clear;
use vapors_text::FontSystem;

const SHEET: &str =
    "html, body { margin: 0; } gpu-surface { display: block; width: 64px; height: 48px; }";

/// The test page: a document with one `gpu-surface` named `track`.
struct Page {
    document: Rc<RefCell<Document>>,
    app: ElementsApp,
    surface: NodeId,
}
impl Page {
    fn new() -> Self {
        let mut document = Document::new(FontSystem::bundle_only());
        document.add_stylesheet("page.css", SHEET).expect("parses");
        let surface = add_surface(&mut document);
        let document = Rc::new(RefCell::new(document));
        let app = ElementsApp::new(Rc::clone(&document));
        Self {
            document,
            app,
            surface,
        }
    }
    fn add_surface(&mut self) {
        self.surface = add_surface(&mut self.document.borrow_mut());
    }
    /// Mounts a new `Track`: its handle and log.
    fn mount(&mut self) -> (SurfaceHandle, Rc<TrackLog>) {
        let log = Rc::new(TrackLog::default());
        let scene = Rc::new(RefCell::new(Scene::default()));
        let track = Box::new(Track::new(scene, Rc::clone(&log)));
        (self.app.mount("track", track).expect("mounts"), log)
    }
    /// Mounts a new `Track`, animating: its handle and log.
    fn mount_animating(&mut self) -> (SurfaceHandle, Rc<TrackLog>) {
        let (handle, log) = self.mount();
        handle.request_animation().expect("animates");
        (handle, log)
    }
    /// One frame that must not fail: what it asked for next.
    fn frame(&mut self, gpu: &Gpu) -> Next {
        let texture = clear(&gpu.device, &gpu.queue, 64, 48, MAGENTA);
        let next = self.app.frame(&Frame {
            adapter: &gpu.adapter,
            device: &gpu.device,
            queue: &gpu.queue,
            texture: &texture,
            index: 1,
            scale_factor: 1.0,
            now: Instant::now(),
        });
        assert_eq!(self.app.failure().get(), None, "the frame failed");
        next
    }
    /// Removes the surface element and unmounts its renderer, mid-animation.
    fn remove_track(&mut self) {
        self.document.borrow_mut().remove(self.surface);
        let renderer = self.app.unmount("track").expect("mounted");
        drop(renderer);
    }
}

/// Appends a `gpu-surface` named `track` to `doc`'s body.
fn add_surface(doc: &mut Document) -> NodeId {
    let surface = doc.create_element("gpu-surface");
    doc.set_attribute(surface, "name", "track");
    let body = doc.body();
    doc.append(body, surface);
    surface
}

/// A page whose track has animated for two frames.
fn animated(gpu: &Gpu) -> (Page, SurfaceHandle, Rc<TrackLog>) {
    let mut page = Page::new();
    let (handle, log) = page.mount_animating();
    assert_eq!([page.frame(gpu), page.frame(gpu)], [Next::Continue; 2]);
    assert_eq!(log.renders.get(), 2);
    (page, handle, log)
}

#[test]
fn gpu_unmounting_the_track_mid_animation_drops_its_renderer_and_the_app_waits() {
    // Given
    let gpu = gpu();
    let (mut page, _handle, log) = animated(&gpu);
    assert!(!log.dropped.get());

    // When
    page.remove_track();
    let after = [page.frame(&gpu), page.frame(&gpu)];

    // Then: the removal changed the document, so its frame asks for
    // one more, as any document change does; nothing animates after it
    assert!(log.dropped.get(), "unmount kept the renderer alive");
    assert_eq!(after, [Next::Continue, Next::Wait]);
    assert_eq!(log.renders.get(), 2);
}

#[test]
fn gpu_an_unmounted_track_s_handle_refuses_every_call_as_not_mounted() {
    // Given
    let gpu = gpu();
    let (mut page, handle, _log) = animated(&gpu);

    // When
    page.remove_track();

    // Then
    let refused = || InvalidateError::NotMounted("track".into());
    assert_eq!(handle.request_animation(), Err(refused()));
    assert_eq!(handle.invalidate(), Err(refused()));
    assert_eq!(handle.stop_animation(), Err(refused()));
    let message = refused().to_string();
    assert!(message.contains("\"track\" has no renderer"), "{message}");
    assert_eq!(
        [page.frame(&gpu), page.frame(&gpu)],
        [Next::Continue, Next::Wait]
    );
}

#[test]
fn gpu_a_new_track_mounted_after_an_unmount_renders_on_a_new_surface() {
    // Given
    let gpu = gpu();
    let (mut page, _handle, _log) = animated(&gpu);
    page.remove_track();
    page.frame(&gpu);

    // When
    page.add_surface();
    let (_handle, log) = page.mount_animating();
    let next = [page.frame(&gpu), page.frame(&gpu)];

    // Then
    assert_eq!(next, [Next::Continue; 2]);
    assert_eq!((log.resizes.get(), log.renders.get()), (1, 2));
    assert_eq!(log.depth.get(), Some([64, 48]));
}

#[test]
fn gpu_a_track_mounted_after_an_unmount_mid_animation_does_not_inherit_the_animation() {
    // Given
    let gpu = gpu();
    let (mut page, _handle, _log) = animated(&gpu);
    page.remove_track();
    page.frame(&gpu);

    // When: mounted with no request
    page.add_surface();
    let (_handle, log) = page.mount();
    let next = [page.frame(&gpu), page.frame(&gpu)];

    // Then: its first paint renders it; nothing asks for more
    assert_eq!(next, [Next::Continue, Next::Wait]);
    assert_eq!(log.renders.get(), 1);
}
