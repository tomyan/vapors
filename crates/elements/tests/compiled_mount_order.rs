//! Compiled winners leave ties between sheets to the cascade's real
//! order (build-time-styles.md, B3a), so two components mounted in
//! either order style their elements as the full cascade does: the
//! debug oracle compares every element, and the styles are pinned here.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, viewport};
use vapors_compiler_examples::counter::{Counter, CounterProps};
use vapors_compiler_examples::flex_body::{FlexBody, FlexBodyProps};
use vapors_css::cascade::ComputedStyle;
use vapors_elements::UpdateCounts;

/// The body's and the button's computed values (not where they were
/// declared: the sheets' indices follow the mount order), and the
/// counts, after mounting the flex body sheet before or after the
/// counter.
fn styled(sheet_first: bool) -> (Vec<Vec<String>>, UpdateCounts) {
    let doc = Rc::new(RefCell::new(document()));
    let (root, body) = (doc.borrow().root(), doc.borrow().body());
    let sheet = || FlexBody::mount(&doc, root, FlexBodyProps::default());
    let sheet = sheet_first.then(sheet);
    let _counter = Counter::mount(&doc, body, CounterProps::default());
    let _sheet = sheet.unwrap_or_else(|| FlexBody::mount(&doc, root, FlexBodyProps::default()));
    doc.borrow_mut()
        .update(viewport(200.0, 100.0))
        .expect("updates");
    let doc = doc.borrow();
    let button = doc.children(body)[0];
    let style = |node| values(doc.computed_style(node).expect("styled"));
    (vec![style(body), style(button)], doc.update_counts())
}

fn values(style: &ComputedStyle) -> Vec<String> {
    style.iter().map(|(_, value)| value.to_css()).collect()
}

#[test]
fn two_components_mounted_in_either_order_style_alike_from_compiled_winners() {
    // When
    let (before, counts) = styled(true);
    let (after, _) = styled(false);

    // Then
    assert!(before == after, "the styles depend on the mount order");
    assert_eq!(counts.winners_runtime, 0, "{counts:?}");
    assert!(counts.winners_compiled >= 1, "{counts:?}");
}

#[test]
fn a_compiled_page_takes_every_keyed_style_from_its_winners() {
    // When
    let (_, counts) = styled(true);

    // Then: the button; flex_body builds nothing, and html and body are unkeyed
    assert_eq!(counts.winners_compiled, 1, "{counts:?}");
    assert_eq!(
        counts.winners_runtime + counts.winners_component_tag,
        0,
        "{counts:?}"
    );
}

/// The colour of each `span.tie`, after mounting `tie_red` then `tie_blue`,
/// or the reverse: each component's own rule and the other's, of equal
/// specificity, tie on both spans, and the later sheet wins.
fn tie(red_first: bool) -> (Vec<String>, UpdateCounts) {
    use vapors_compiler_examples::tie_blue::{TieBlue, TieBlueProps};
    use vapors_compiler_examples::tie_red::{TieRed, TieRedProps};
    let doc = Rc::new(RefCell::new(document()));
    let body = doc.borrow().body();
    let red = || TieRed::mount(&doc, body, TieRedProps::default());
    let blue = || TieBlue::mount(&doc, body, TieBlueProps::default());
    let (_red, _blue) = if red_first {
        (red(), blue())
    } else {
        let b = blue();
        (red(), b)
    };
    doc.borrow_mut()
        .update(viewport(200.0, 100.0))
        .expect("updates");
    let doc = doc.borrow();
    let colour = |node| {
        let style = doc.computed_style(node).expect("styled");
        style.get_by_name("color").expect("a longhand").to_css()
    };
    (
        doc.children(body).into_iter().map(colour).collect(),
        doc.update_counts(),
    )
}

// Both tie sheets are compiler-examples', so each key's winners list
// both and neither is another crate's (`winners_foreign` stays 0): the tie
// runs through the tested rules. A sheet of another crate is covered at
// the css level (`cascade_compiled.rs`); an elements-level case waits on a
// second compiled crate (#445).
#[test]
fn a_tie_between_two_components_sheets_goes_to_the_one_mounted_later_in_either_order() {
    // When
    let (red_then_blue, counts) = tie(true);
    let (blue_then_red, _) = tie(false);

    // Then: both spans, each from compiled winners
    assert_eq!(red_then_blue, vec!["rgb(0 0 255)"; 2]);
    assert_eq!(blue_then_red, vec!["rgb(255 0 0)"; 2]);
    assert_eq!(counts.winners_compiled, 2, "{counts:?}");
}
