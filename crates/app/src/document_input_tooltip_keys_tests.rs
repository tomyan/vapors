use vapors_dom::Dom;
use vapors_platform::Event;

use super::tests::{give, key, page, press, shown, update};
use crate::DocumentInput;
use crate::tooltip::TooltipTrigger;
use vapors_elements::Document;

#[test]
fn tab_to_a_title_shows_it_after_the_delay_and_esc_hides_it() {
    // Given
    let (doc, open) = page();
    let mut input = DocumentInput::new();

    // When
    give(&mut input, &doc, &key("Tab", "Tab"));

    // Then
    assert_eq!(doc.borrow().active_element(), Some(open));
    let deadline = input.tooltip_deadline().expect("armed by focus");
    let tip = input.tooltip(&doc.borrow(), deadline).cloned();
    assert_eq!(tip.map(|tip| tip.trigger), Some(TooltipTrigger::Keyboard));
    give(&mut input, &doc, &key("Escape", "Escape"));
    assert_eq!(shown(&mut input, &doc, deadline), None);
}

#[test]
fn focus_a_press_gave_does_not_arm_it_even_when_focus_visible() {
    // Given: a text field, which a press makes :focus-visible
    let (doc, _) = page();
    let mut input = DocumentInput::new();
    give(&mut input, &doc, &Event::PointerMoved { x: 250.0, y: 10.0 });

    // When
    give(&mut input, &doc, &press(true));
    give(&mut input, &doc, &press(false));

    // Then
    assert!(doc.borrow().focus_visible());
    assert_eq!(input.tooltip_deadline(), None);
}

#[test]
fn a_keyboard_tooltip_hides_when_a_script_blurs_its_element() {
    // Given
    let (doc, _) = page();
    let mut input = DocumentInput::new();
    give(&mut input, &doc, &key("Tab", "Tab"));
    let deadline = input.tooltip_deadline().expect("armed by focus");

    // When
    Document::focus(&doc, None);

    // Then
    assert_eq!(shown(&mut input, &doc, deadline), None);
}

#[test]
fn a_keyboard_tooltip_hides_when_its_element_loses_its_box() {
    // Given
    let (doc, open) = page();
    let mut input = DocumentInput::new();
    give(&mut input, &doc, &key("Tab", "Tab"));
    let deadline = input.tooltip_deadline().expect("armed by focus");

    // When
    doc.borrow_mut()
        .set_attribute(open, "style", "display: none");
    update(&mut doc.borrow_mut());

    // Then
    assert_eq!(doc.borrow().active_element(), Some(open), "still focused");
    assert_eq!(shown(&mut input, &doc, deadline), None);
}

#[test]
fn a_keyboard_tooltip_hides_when_its_element_is_disabled_and_loses_focus() {
    // Given
    let (doc, open) = page();
    let mut input = DocumentInput::new();
    give(&mut input, &doc, &key("Tab", "Tab"));
    let deadline = input.tooltip_deadline().expect("armed by focus");

    // When: disabled, which keeps its box, then the focus fixup
    doc.borrow_mut().set_attribute(open, "disabled", "");
    update(&mut doc.borrow_mut());
    Document::settle_focus(&doc);

    // Then
    assert_eq!(doc.borrow().active_element(), None);
    assert_eq!(shown(&mut input, &doc, deadline), None);
}
