//! The IU1 rows workload, timed: `rows` mounted rows (the page of
//! vapors-elements' `sparse_updates.rs`, its files included by path so
//! both time and check one workload) in an `ElementsApp`, and a change
//! made before each frame.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{ElementsApp, FrameTiming};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Next};

use crate::run::{Counts, Workload};

#[path = "../../elements/tests/support/basics.rs"]
mod basics;
#[allow(dead_code, clippy::pedantic)] // the tests' file: `edit` and its other edits are theirs
#[path = "../../elements/tests/support/rows_page.rs"]
mod rows_page;
pub use rows_page::rows_in_view;
use rows_page::{Edit, RowsPage, row, rows_page, rows_viewport};

/// What changes before each frame.
#[derive(Clone, Copy, Debug)]
pub enum RowChange {
    /// Nothing: the frame updates and draws an unchanged document.
    Idle,
    /// The middle row's value, to a new five-digit text: no box moves.
    Text,
    /// The middle row's `marked` class added or removed: its background.
    Class,
    /// `Class` with the pointer over the second row's button, so each
    /// update also hit-tests the pointer's position (review v201 M2).
    HoveredClass,
    /// Alternating frames: a row inserted before the middle row, then
    /// that row removed. Starts with an insert, warmup frames counted.
    Insert,
    /// The second row's height, between one and two rows: its boxes
    /// are built again and every row after it moves (IU3 3c-3e).
    Resize,
    /// The body, which scrolls its rows, scrolled down one row; back to
    /// the top at the end. Every row's boxes move.
    Scroll,
}

impl RowChange {
    /// Every change, in the order the harness lists them.
    pub const ALL: [Self; 7] = [
        Self::Idle,
        Self::Text,
        Self::Class,
        Self::HoveredClass,
        Self::Insert,
        Self::Resize,
        Self::Scroll,
    ];

    /// The change `name` spells, as `{:?}` prints it.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|change| format!("{change:?}") == name)
    }
}

/// A rows page in an app, and its change.
pub struct Rows {
    page: RowsPage,
    app: ElementsApp,
    change: RowChange,
    /// The row `Insert` added and has not yet removed.
    inserted: Option<NodeId>,
}

impl Rows {
    /// `count` rows, updated once, in an app that has drawn no frame and
    /// times its frames on the GPU.
    #[must_use]
    pub fn new(count: usize, change: RowChange) -> Self {
        let page = rows_page(count);
        match change {
            RowChange::HoveredClass => page.edit(Edit::Hover),
            RowChange::Resize => page.rows_resizable(),
            RowChange::Scroll => page.body_scrolls(),
            RowChange::Idle | RowChange::Text | RowChange::Class | RowChange::Insert => {}
        }
        let mut app = ElementsApp::new(Rc::clone(&page.doc));
        app.time_frames(true);
        Self {
            page,
            app,
            change,
            inserted: None,
        }
    }
}

impl Rows {
    /// Removes the inserted row, or inserts one before the middle row.
    fn toggle_inserted_row(&mut self) {
        let mut doc = self.page.doc.borrow_mut();
        if let Some(inserted) = self.inserted.take() {
            doc.remove(inserted);
            return;
        }
        let middle = self.page.rows[self.page.rows.len() / 2].element;
        let added = row(&mut doc, self.page.rows.len()).element;
        let body = doc.body();
        doc.insert_before(body, added, Some(middle));
        self.inserted = Some(added);
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
            RowChange::Class | RowChange::HoveredClass => self.page.edit(Edit::ToggleClass),
            RowChange::Insert => self.toggle_inserted_row(),
            RowChange::Resize => self.page.toggle_tall(),
            RowChange::Scroll => {
                if !self.page.scroll_body(1.0) {
                    self.page.scroll_body_to_top();
                }
            }
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

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
