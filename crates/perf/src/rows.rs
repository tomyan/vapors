//! The IU1 rows workload, timed: `rows` mounted rows (the page of
//! vapors-elements' `sparse_updates.rs`, its files included by path so
//! both time and check one workload) in an `ElementsApp`, and a change
//! made before each frame.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{ElementsApp, FrameTiming};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Next};

use crate::run::{Counts, Workload};

#[path = "../../elements/tests/support/basics.rs"]
mod basics;
#[allow(dead_code, clippy::pedantic)] // the tests' file: `edit` and its other edits are theirs
#[path = "../../elements/tests/support/rows_page.rs"]
mod rows_page;
pub use rows_page::rows_in_view;
use rows_page::{Edit, RowsPage, rows_page, rows_viewport};

/// What changes before each frame.
#[derive(Clone, Copy, Debug)]
pub enum RowChange {
    /// Nothing: the frame updates and draws an unchanged document.
    Idle,
    /// The middle row's value, to a new five-digit text: no box moves.
    Text,
    /// The middle row's `marked` class added or removed: its background.
    Class,
}

/// A rows page in an app, and its change.
pub struct Rows {
    page: RowsPage,
    app: ElementsApp,
    change: RowChange,
}

impl Rows {
    /// `count` rows, updated once, in an app that has drawn no frame and
    /// times its frames on the GPU.
    #[must_use]
    pub fn new(count: usize, change: RowChange) -> Self {
        let page = rows_page(count);
        let mut app = ElementsApp::new(Rc::clone(&page.doc));
        app.time_frames(true);
        Self { page, app, change }
    }
}

impl Workload for Rows {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 640 x 400 at 1x
    fn size(&self) -> (u32, u32) {
        let viewport = rows_viewport();
        (viewport.width as u32, viewport.height as u32)
    }

    fn change(&mut self, sample: usize) {
        match self.change {
            RowChange::Idle => {}
            RowChange::Text => {
                let value = self.page.rows[self.page.rows.len() / 2].value;
                let text = format!("{:05}", 90_000 + sample % 10_000);
                self.page.doc.borrow_mut().set_text(value, &text);
            }
            RowChange::Class => self.page.edit(Edit::ToggleClass),
        }
    }

    fn app(&mut self) -> &mut dyn App {
        &mut self.app
    }

    fn check(&self, next: Next) -> Result<(), String> {
        match (next, self.app.failure().get()) {
            (_, Some(failure)) => Err(failure),
            (Next::Exit, None) => Err("the app exited without a failure".into()),
            _ => Ok(()),
        }
    }

    fn counts(&self) -> Counts {
        let frame = self.app.frame_counts();
        Counts::of(&self.page.doc.borrow(), frame, self.app.render_passes())
    }

    fn timing(&self) -> FrameTiming {
        self.app.frame_timing()
    }
}

/// `count` rows updated once, and the viewport they update at, for the
/// phase clock's tests.
#[must_use]
pub fn rows_document(count: usize) -> (Rc<RefCell<Document>>, Viewport) {
    (rows_page(count).doc, rows_viewport())
}
