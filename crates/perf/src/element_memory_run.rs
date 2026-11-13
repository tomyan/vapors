//! One page measured for `element_memory` in this process: the live
//! heap before the mount, after it, and after the app's frames settle
//! (it asks for no next frame), each frame's phases metered by
//! `HeapPhases`. One size to a process: memory a page frees stays with
//! the allocator for the next one, but its counts do not, and fonts and
//! pipelines made once would be charged to the first size alone.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_gpu_harness::Gpu;
use vapors_platform::{App, Frame, Next};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_signals::Signal;
use vapors_text::FontSystem;

use crate::element_memory::ElementMemory;
use crate::gpu_time::wait;
use crate::heap_phases::HeapPhases;
use crate::mount::row_data;
use crate::mount_rows::{MountRows, MountRowsProps};

/// The frame's device size, as the Mount workload's.
const SIZE: (u32, u32) = (640, 400);

/// Frames a page may take to settle. A mounted page asks for no frame
/// after its first unless an effect or a surface does; ten is room for
/// either, and a page still asking is refused, not cut off.
const SETTLE_FRAMES: u64 = 10;

/// How long the GPU may take with one frame (as the sampler's).
const FRAME_DEADLINE: Duration = Duration::from_secs(10);

/// `rows` rows mounted on a fresh page and settled on `gpu`, the live
/// heap read from `live`.
///
/// # Errors
/// A frame the app failed, a GPU wait that failed, or a page that still
/// asks for frames after [`SETTLE_FRAMES`].
pub fn run_child(gpu: &Gpu, rows: usize, live: fn() -> i64) -> Result<ElementMemory, String> {
    let phases = HeapPhases::new(live);
    let texture = clear(&gpu.device, &gpu.queue, SIZE.0, SIZE.1, Color::default());
    let before = live();
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let _rows = MountRows::mount(
        &doc,
        body,
        MountRowsProps {
            rows: Signal::new(row_data(rows)),
        },
    );
    let mount = live() - before;
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let dispatch = tracing::Dispatch::new(phases.clone());
    let counts =
        tracing::dispatcher::with_default(&dispatch, || settle(gpu, (&mut app, &doc), &texture))?;
    let total = live() - before;
    Ok(ElementMemory {
        rows,
        elements: counts.0,
        styles_computed: counts.1,
        total,
        owners: ElementMemory::split(mount, &phases.take(), total),
    })
}

/// The app's frames until it asks for none, returning the first
/// update's element and computed-style counts.
fn settle(
    gpu: &Gpu,
    (app, doc): (&mut ElementsApp, &RefCell<Document>),
    texture: &wgpu::Texture,
) -> Result<(usize, usize), String> {
    let mut first = None;
    let mut now = vapors_platform::Instant::now();
    for index in 0..SETTLE_FRAMES {
        let next = app.frame(&frame(gpu, texture, index, now));
        wait(&gpu.device, FRAME_DEADLINE)?;
        if let Some(failure) = app.failure().get() {
            return Err(format!("frame {index}: {failure}"));
        }
        let counts = doc.borrow().update_counts();
        let first = *first.get_or_insert((counts.elements_visited, counts.styles_computed));
        if next == Next::Wait {
            return Ok(first);
        }
        now += Duration::from_millis(16);
    }
    Err(format!(
        "the page still asks for frames after {SETTLE_FRAMES}; its memory is measured settled, \
         so find what asks (an effect or an animating surface) before measuring"
    ))
}

fn frame<'a>(
    gpu: &'a Gpu,
    texture: &'a wgpu::Texture,
    index: u64,
    now: vapors_platform::Instant,
) -> Frame<'a> {
    Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture,
        index,
        scale_factor: 1.0,
        now,
    }
}
