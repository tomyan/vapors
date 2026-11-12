//! The page a row change runs on: the rows page of `sparse_updates.rs`
//! or, for `FlexText`, the flex-column list of `sparse_updates_flex.rs`
//! (M10c), whose edited row is a flex item its parent asks for its
//! size before laying it out. Both files are the elements tests', so
//! the harness times what they check.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::Document;

pub use crate::rows::flex_list::list_viewport;
use crate::rows::flex_list::{FlexList, ROW_MARGIN, flex_list};
use crate::rows::rows_page::RowsPage;

/// The document a change edits.
pub enum Page {
    /// The rows page: a label, a value and a button per flex row.
    Rows(RowsPage),
    /// The flex-column list: a block row holding its text per row.
    List(FlexList),
}

impl Page {
    /// `count` rows of the flex-column list, with the margin that
    /// makes each row's size answer one Taffy's cache keeps no entry
    /// for.
    #[must_use]
    pub fn list(count: usize) -> Self {
        Self::List(flex_list(count, ROW_MARGIN))
    }

    /// The document the page is.
    #[must_use]
    pub fn doc(&self) -> &Rc<RefCell<Document>> {
        match self {
            Self::Rows(page) => &page.doc,
            Self::List(list) => &list.doc,
        }
    }

    /// The rows page; a change made to the list has none to edit.
    #[must_use]
    pub fn rows(&self) -> &RowsPage {
        match self {
            Self::Rows(page) => page,
            Self::List(_) => unreachable!("FlexText runs on the flex list, which has no rows page"),
        }
    }

    /// Sets the list's middle row to a new five-digit value.
    pub fn set_list_value(&self, sample: usize) {
        match self {
            Self::List(list) => list.set_middle_value(90_000 + sample % 10_000),
            Self::Rows(_) => unreachable!("the rows page has no list value to set"),
        }
    }
}
