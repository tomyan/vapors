//! The Mount workload: a compiled `{for}` of row components mounted
//! under the body of a fresh document, then the app's first frame. The
//! sampler times that frame (its `update` and `style` spans are the
//! first update's); the mount before it is timed and counted here, and
//! the resident set is read before and after it. Every sample after the
//! first mounts a fresh page, so each frame is a first frame.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vapors_app::{ElementsApp, FrameTiming};
use vapors_elements::Document;
use vapors_platform::{App, Next};
use vapors_signals::Signal;
use vapors_text::FontSystem;

use crate::instructions::InstructionCounter;
use crate::rss::{peak_kib, resident_kib};
use crate::run::{Counts, Workload};

use crate::mount_rows::{MountRows, MountRowsProps};

/// The frame's device size, as the rows workload's.
const SIZE: (u32, u32) = (640, 400);

/// What mounting one page cost.
#[derive(Clone, Copy, Debug)]
pub struct Mounted {
    /// Wall time of `MountRows::mount`.
    pub time: Duration,
    /// User-space instructions of the same call.
    pub instructions: u64,
    /// The resident set before the mount, KiB.
    pub rss_before_kib: u64,
    /// The resident set after it, KiB.
    pub rss_after_kib: u64,
}

/// The resident set after a page's first frame.
#[derive(Clone, Copy, Debug)]
pub struct AfterFrame {
    /// KiB now.
    pub resident_kib: u64,
    /// The most KiB the process has held.
    pub peak_kib: u64,
}

/// A mounted page; the component is kept so its effects live.
struct Page {
    app: ElementsApp,
    doc: Rc<RefCell<Document>>,
    _rows: MountRows,
}

/// `count` compiled rows, mounted again before each frame.
pub struct Mount {
    count: usize,
    counter: InstructionCounter,
    page: Option<Page>,
    /// One entry per page mounted, warmup included.
    pub mounted: Vec<Mounted>,
    /// One entry per frame checked, read by [`Workload::check`].
    pub after_frame: RefCell<Vec<AfterFrame>>,
    /// The first mount that could not be measured; the next check fails.
    failure: Option<String>,
}

impl Mount {
    /// A workload of `count` rows, counting mounts with `counter`.
    #[must_use]
    pub fn new(count: usize, counter: InstructionCounter) -> Self {
        let mut mount = Self {
            count,
            counter,
            page: None,
            mounted: Vec::new(),
            after_frame: RefCell::default(),
            failure: None,
        };
        mount.remount();
        mount
    }

    fn remount(&mut self) {
        self.page = None;
        match mount_page(self.count, &mut self.counter) {
            Ok((page, mounted)) => {
                self.page = Some(page);
                self.mounted.push(mounted);
            }
            Err(error) => self.failure = self.failure.take().or(Some(error)),
        }
    }

    fn page(&mut self) -> &mut Page {
        self.page.as_mut().expect("check refuses a failed mount")
    }
}

/// The rows' data: an id, a label and a value each.
fn row_data(count: usize) -> Vec<(u32, String, String)> {
    (0..count)
        .map(|i| {
            (
                u32::try_from(i).expect("fewer than 2^32 rows"),
                format!("Row {i}"),
                format!("{:05}", i * 7),
            )
        })
        .collect()
}

fn mount_page(count: usize, counter: &mut InstructionCounter) -> Result<(Page, Mounted), String> {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let rows = Signal::new(row_data(count));
    let rss_before_kib = resident_kib()?;
    let (started, before) = (Instant::now(), counter.read()?);
    let mounted_rows = MountRows::mount(&doc, body, MountRowsProps { rows });
    let (time, after) = (started.elapsed(), counter.read()?);
    let rss_after_kib = resident_kib()?;
    let app = ElementsApp::new(Rc::clone(&doc));
    let mounted = Mounted {
        time,
        instructions: after - before,
        rss_before_kib,
        rss_after_kib,
    };
    Ok((
        Page {
            app,
            doc,
            _rows: mounted_rows,
        },
        mounted,
    ))
}

impl Workload for Mount {
    fn size(&self) -> (u32, u32) {
        SIZE
    }

    fn change(&mut self, sample: usize) {
        if sample > 0 {
            self.remount();
        }
    }

    fn app(&mut self) -> &mut dyn App {
        &mut self.page().app
    }

    fn check(&self, next: Next) -> Result<(), String> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        let page = self.page.as_ref().ok_or("no page is mounted")?;
        self.after_frame.borrow_mut().push(AfterFrame {
            resident_kib: resident_kib()?,
            peak_kib: peak_kib()?,
        });
        match (next, page.app.failure().get()) {
            (_, Some(failure)) => Err(failure),
            (Next::Exit, None) => Err("the app exited without a failure".into()),
            _ => Ok(()),
        }
    }

    fn counts(&self) -> Counts {
        let page = self.page.as_ref().expect("check refuses a failed mount");
        Counts::of(
            &page.doc.borrow(),
            page.app.frame_counts(),
            page.app.render_passes(),
        )
    }

    /// Each frame is a new app's first: no later frame reads a time back.
    fn times_gpu(&self) -> bool {
        false
    }

    fn timing(&self) -> FrameTiming {
        self.page
            .as_ref()
            .expect("check refuses a failed mount")
            .app
            .frame_timing()
    }
}

#[cfg(test)]
#[path = "mount_tests.rs"]
mod tests;
