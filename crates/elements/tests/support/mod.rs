//! Shared by the elements tests: a document on the bundled fonts only,
//! so results do not depend on the machine's fonts.

#![allow(dead_code)] // each test file uses a subset

pub mod basics;
pub mod changes;
pub mod checkbox_page;
pub mod column_page;
pub mod drive;
pub mod fixup;
pub mod focus_page;
pub mod hidden_page;
pub mod mirror;
pub mod number_page;
pub mod oracle;
pub mod pointer;
pub mod random_css;
pub mod range_page;
pub mod roles_page;
pub mod rows_page;
pub mod scroll_list;
pub mod splice_page;
pub mod svg_page;
pub mod sweep_page;
pub mod sweep_paths;
pub mod text_page;
pub mod tree;

pub use basics::{document, element, text, viewport};
use vapors_elements::{Document, ElementError, Viewport};

/// The update's errors, each as its message; panics when it succeeds.
pub fn refusals(doc: &mut Document) -> Vec<String> {
    refusals_at(doc, viewport(400.0, 300.0))
}

/// The messages of the refusal of `doc` updated at `viewport`.
pub fn refusals_at(doc: &mut Document, viewport: Viewport) -> Vec<String> {
    match doc.update(viewport) {
        Ok(_) => panic!("the update succeeded; a refusal was expected"),
        Err(errors) => errors.iter().map(ElementError::to_string).collect(),
    }
}
