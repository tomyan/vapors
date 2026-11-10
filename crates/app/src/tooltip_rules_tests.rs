//! What a move while the button is down and a move within the hovered
//! title keep, and when what is armed or shown counts as changed.

use vapors_dom::NodeId;
use vapors_platform::Instant;

use super::TooltipTrigger;
use super::tests::{armed, ms, titled};

#[test]
fn a_drag_that_ends_on_the_pressed_title_keeps_it_hidden() {
    // Given
    let t0 = Instant::now();
    let mut state = armed(t0);
    state.pressed();

    // When
    state.pointer_over(Some(titled(1, "Open (o)")), t0 + ms(100));
    state.released();
    state.pointer_over(Some(titled(1, "Open (o)")), t0 + ms(200));

    // Then
    assert_eq!(state.deadline(), None);
}

#[test]
fn a_move_within_the_hovered_title_keeps_a_keyboard_tooltip() {
    // Given: the pointer over node 1, then a key moved focus to node 3
    let t0 = Instant::now();
    let mut state = armed(t0);
    state.focus(Some(NodeId(3)), Some(titled(3, "Save")), t0 + ms(100));

    // When
    state.pointer_over(Some(titled(1, "Open (o)")), t0 + ms(200));

    // Then
    let shown = state.shown_at(t0 + ms(600)).cloned();
    assert_eq!(shown.map(|tip| tip.trigger), Some(TooltipTrigger::Keyboard));
}

#[test]
fn arming_showing_and_hiding_each_count_as_a_change_once() {
    // Given
    let t0 = Instant::now();
    let mut state = armed(t0);

    // Then
    assert!(state.take_changed(), "armed");
    assert!(!state.take_changed(), "taken");
    state.pointer_over(Some(titled(1, "Open (o)")), t0 + ms(100));
    assert!(!state.take_changed(), "a move within the title");
    assert!(state.shown_at(t0 + ms(500)).is_some());
    assert!(state.take_changed(), "shown");
    assert!(state.shown_at(t0 + ms(600)).is_some());
    assert!(!state.take_changed(), "still shown");
    state.hide();
    assert!(state.take_changed(), "hidden");
    state.hide();
    assert!(!state.take_changed(), "nothing to hide");
}

#[test]
fn a_new_title_arming_counts_as_a_change() {
    // Given
    let t0 = Instant::now();
    let mut state = armed(t0);
    state.take_changed();

    // When
    state.pointer_over(Some(titled(2, "Undo")), t0 + ms(100));

    // Then
    assert!(state.take_changed());
}
