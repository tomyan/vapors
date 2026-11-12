//! The IU1 rows workload, timed: `rows` mounted rows (the page of
//! vapors-elements' `sparse_updates.rs`, its files included by path so
//! both time and check one workload; `rows_list.rs` for the flex list
//! of `FlexText`) in an `ElementsApp`, and a change made before each
//! frame.

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
pub(crate) mod rows_page;
pub use rows_page::rows_in_view;
use rows_page::{Edit, row, rows_page, rows_viewport};
#[allow(dead_code, clippy::pedantic)] // the tests' file: `set_middle_text` and `row_text` are theirs
#[path = "../../elements/tests/support/flex_list.rs"]
pub(crate) mod flex_list;

use crate::rows_list::Page;

/// What changes before each frame.
#[derive(Clone, Copy, Debug)]
pub enum RowChange {
    /// Nothing: the frame updates and draws an unchanged document.
    Idle,
    /// The middle row's value, to a new five-digit text: no box moves.
    Text,
    /// `Text` on the flex-column list (`rows_list.rs`): the edited row
    /// is a flex item, asked for its size before it is laid out.
    FlexText,
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
    pub const ALL: [Self; 8] = [
        Self::Idle,
        Self::Text,
        Self::FlexText,
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
    page: Page,
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
        let page = match change {
            RowChange::FlexText => Page::list(count),
            _ => Page::Rows(rows_page(count)),
        };
        match change {
            RowChange::HoveredClass => page.rows().edit(Edit::Hover),
            RowChange::Resize => page.rows().rows_resizable(),
            RowChange::Scroll => page.rows().body_scrolls(),
            RowChange::Idle
            | RowChange::Text
            | RowChange::FlexText
            | RowChange::Class
            | RowChange::Insert => {}
        }
        let mut app = ElementsApp::new(Rc::clone(page.doc()));
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
        let page = self.page.rows();
        let mut doc = page.doc.borrow_mut();
        if let Some(inserted) = self.inserted.take() {
            doc.remove(inserted);
            return;
        }
        let middle = page.rows[page.rows.len() / 2].element;
        let added = row(&mut doc, page.rows.len()).element;
        let body = doc.body();
        doc.insert_before(body, added, Some(middle));
        self.inserted = Some(added);
    }
}

impl Workload for Rows {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 640 x 400 at 1x
    fn size(&self) -> (u32, u32) {
        let viewport = match self.page {
            Page::Rows(_) => rows_viewport(),
            Page::List(_) => crate::rows_list::list_viewport(),
        };
        (viewport.width as u32, viewport.height as u32)
    }

    fn change(&mut self, sample: usize) {
        match self.change {
            RowChange::Idle => {}
            RowChange::Text => {
                let page = self.page.rows();
                let value = page.rows[page.rows.len() / 2].value;
                let text = format!("{:05}", 90_000 + sample % 10_000);
                page.doc.borrow_mut().set_text(value, &text);
            }
            RowChange::FlexText => self.page.set_list_value(sample),
            RowChange::Class | RowChange::HoveredClass => self.page.rows().edit(Edit::ToggleClass),
            RowChange::Insert => self.toggle_inserted_row(),
            RowChange::Resize => self.page.rows().toggle_tall(),
            RowChange::Scroll => {
                let page = self.page.rows();
                if !page.scroll_body(1.0) {
                    page.scroll_body_to_top();
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
        Counts::of(&self.page.doc().borrow(), frame, self.app.render_passes())
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
