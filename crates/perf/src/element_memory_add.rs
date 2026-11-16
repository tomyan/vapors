//! One structural insert on a settled page (property 8, the review of
//! #333, M3): the live bytes one row added leaves, and the wall time of
//! the frames that show it, so a growth of the kept box list (by a
//! quarter, `slots_subtree.rs`) shows its bytes and its copy's cost.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_gpu_harness::Gpu;
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_signals::Signal;
use vapors_text::FontSystem;

use crate::element_memory_run::{SIZE, settle};
use crate::mount::row_data;
use crate::mount_rows::{MountRows, MountRowsProps};

/// What one row added to a settled page of `rows` rows cost.
#[derive(Clone, Copy, Debug)]
pub struct AddedRow {
    /// The page's rows before the insert.
    pub rows: usize,
    /// Live heap bytes after the insert's frames less before the insert.
    pub bytes: i64,
    /// Wall time of the frames until the page settles again.
    pub frames: Duration,
}

impl AddedRow {
    /// The line the memory binary prints for it.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "element-memory-add rows={} bytes={} frames_us={}",
            self.rows,
            self.bytes,
            self.frames.as_micros()
        )
    }
}

/// `rows` rows mounted and settled on `gpu`, then one row added at the
/// middle and settled again, the live heap read from `live`.
///
/// # Errors
/// A frame the app failed or a page that does not settle (`settle`).
///
/// # Panics
/// With 2^32 rows or more, which no page has.
pub fn add_row(gpu: &Gpu, rows: usize, live: fn() -> i64) -> Result<AddedRow, String> {
    let texture = clear(&gpu.device, &gpu.queue, SIZE.0, SIZE.1, Color::default());
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let data = Signal::new(row_data(rows));
    let props = MountRowsProps { rows: data };
    let _rows = MountRows::mount(&doc, body, props);
    let mut app = ElementsApp::new(Rc::clone(&doc));
    settle(gpu, (&mut app, &doc), &texture)?;
    let before = live();
    let id = u32::try_from(rows).expect("fewer than 2^32 rows");
    data.update(|rows| rows.insert(rows.len() / 2, (id, "Added".into(), "00000".into())));
    let started = Instant::now();
    settle(gpu, (&mut app, &doc), &texture)?;
    Ok(AddedRow {
        rows,
        bytes: live() - before,
        frames: started.elapsed(),
    })
}
