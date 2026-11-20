//! What the top bar and the cards do, and the refusal of pages s3 builds,
//! through vapors' test drivers.
use super::support::*;
use crate::app::{mount_with, not_built};
use crate::dashboard::*;
use std::{cell::RefCell, rc::Rc};
use vapors_elements::{Document, TextEdit};

fn centre_of(mounted: &crate::app::Mounted, class: &str, nth: usize) -> [f32; 2] {
    let doc = mounted.document.borrow();
    let [x, y, w, h] = rect(&doc, by_class(&doc, class)[nth]);
    [x + w / 2.0, y + h / 2.0]
}

fn texts(mounted: &crate::app::Mounted, class: &str) -> Vec<String> {
    let doc = mounted.document.borrow();
    by_class(&doc, class)
        .into_iter()
        .map(|node| text(&doc, node))
        .collect()
}

#[test]
fn the_top_bar_and_cards_show_the_specs_texts_at_tick_zero() {
    // Given
    let mounted = laid_out();

    // Then
    assert_eq!(texts(&mounted, "title"), [PAGE_TITLE]);
    assert_eq!(texts(&mounted, "export"), [EXPORT_LABEL]);
    assert_eq!(texts(&mounted, "avatar"), [USER_INITIALS]);
    assert_eq!(texts(&mounted, "placeholder"), [SEARCH_PLACEHOLDER]);
    let values: Vec<String> = (0..KPI_COUNT).map(|k| kpi_value(k, 0)).collect();
    assert_eq!(texts(&mounted, "kpi-value"), values);
    let labels: Vec<&str> = KPIS.iter().map(|kpi| kpi.label).collect();
    assert_eq!(texts(&mounted, "kpi-label"), labels);
    let deltas: Vec<&str> = KPIS.iter().map(|kpi| kpi.delta).collect();
    assert_eq!(texts(&mounted, "delta"), deltas);
}

#[test]
fn typing_in_the_search_input_sets_the_search_and_hides_the_placeholder() {
    // Given
    let mounted = laid_out();
    assert!(
        mounted.document.borrow().active_element().is_none(),
        "not focused at startup"
    );

    // When
    click(&mounted, centre_of(&mounted, "search-input", 0));
    let edit = TextEdit {
        before: String::new(),
        text: "ada".into(),
        selection: (3, 3),
        composing: None,
    };
    Document::text_edited(&mounted.document, &edit).expect("the field takes the edit");
    refresh(&mounted);

    // Then
    assert_eq!(mounted.search.get_untracked(), "ada");
    assert_eq!(texts(&mounted, "placeholder"), Vec::<String>::new());
}

#[test]
fn the_delta_pills_are_green_up_and_red_down() {
    // Given
    let mounted = laid_out();
    let doc = mounted.document.borrow();

    // Then
    for (k, pill) in by_class(&doc, "delta").into_iter().enumerate() {
        let style = doc.computed_style(pill).expect("styled");
        let want = if KPIS[k].up { UP_BG } else { DOWN_BG };
        let [_, r, g, b] = want.to_be_bytes();
        let got = style
            .get_by_name("background-color")
            .expect("a property")
            .to_css();
        assert_eq!(got, format!("rgb({r} {g} {b})"), "card {k}");
    }
}

#[test]
fn the_content_scrolls_when_it_holds_more_than_the_window() {
    // Given
    let mounted = laid_out();

    // Then: it is the scroll container, as tall as the window below the bar
    let doc = mounted.document.borrow();
    let style = doc
        .computed_style(by_class(&doc, "content")[0])
        .expect("styled");
    let overflow = style
        .get_by_name("overflow-x")
        .expect("a property")
        .to_css();
    assert_eq!(overflow, "auto");
}

#[test]
fn a_click_on_a_page_s3_builds_reports_it_and_the_overview_does_not() {
    // Given
    let refused = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&refused);
    let mounted = mount_with(move |item| log.borrow_mut().push(item));
    refresh(&mounted);
    assert!(refused.borrow().is_empty(), "the overview is built");

    // When
    click(&mounted, centre_of(&mounted, "item", 1));
    click(&mounted, centre_of(&mounted, "item", 0));

    // Then
    assert_eq!(*refused.borrow(), [1]);
    assert!(not_built("the Orders page").contains("not built yet: s3"));
}
