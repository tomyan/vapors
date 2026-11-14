//! Long lists of fractional rows: a lane sums starts in f64 and Taffy in
//! f32, so far down a list the two differ by more than the last bits;
//! the debug layout check holds them to the f32 running sum's error
//! bound (`layout/layout_check.rs`, `tolerance`). Each page is checked
//! by the update itself.

mod support;

use support::{document, element, text, viewport};

/// 2000 rows of 10.21 px under body styled `body`, updated once: the
/// review's page, where Taffy's body is 20420.313 px and the lane's
/// 20420.0.
fn rows(body_css: &str) {
    let mut doc = document();
    let body = doc.body();
    for index in 0..2000 {
        let row = element(&mut doc, body, "div", "row");
        text(&mut doc, row, &format!("{index}"));
    }
    let css =
        format!("body {{ margin: 0; {body_css} }} .row {{ height: 10.21px; flex-shrink: 0; }}");
    doc.add_stylesheet("drift.css", &css).expect("parses");
    doc.update(viewport(640.0, 400.0)).expect("updates");
}

#[test]
fn a_long_block_list_of_fractional_rows_passes_the_layout_check() {
    rows("");
}

#[test]
fn a_long_flex_column_of_fractional_rows_passes_the_layout_check() {
    rows("display: flex; flex-direction: column;");
}
