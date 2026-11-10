use std::time::Duration;

use vapors_dom::NodeId;
use vapors_platform::Instant;

use super::{TOOLTIP_DELAY, Titled, TooltipState, TooltipTrigger};

pub(super) fn titled(id: u64, text: &str) -> Titled {
    (NodeId(id), text.to_owned())
}

/// A named way to hide the tooltip.
type Hide = (&'static str, fn(&mut TooltipState));

pub(super) fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// A state with the pointer over node 1's title since `t0`.
pub(super) fn armed(t0: Instant) -> TooltipState {
    let mut state = TooltipState::default();
    state.pointer_over(Some(titled(1, "Open (o)")), t0);
    state
}

fn shown_text(state: &mut TooltipState, now: Instant) -> Option<String> {
    state.shown_at(now, None).map(|tip| tip.text.clone())
}

#[test]
fn the_tooltip_shows_at_the_delay_and_not_before() {
    // Given
    let t0 = Instant::now();
    let mut state = armed(t0);

    // Then
    assert_eq!(TOOLTIP_DELAY, ms(500));
    assert_eq!(state.deadline(), Some(t0 + ms(500)));
    assert_eq!(shown_text(&mut state, t0 + ms(499)), None);
    assert_eq!(
        shown_text(&mut state, t0 + ms(500)),
        Some("Open (o)".into())
    );
    assert_eq!(state.deadline(), None, "shown: nothing to wait for");
}

#[test]
fn moving_within_the_same_title_does_not_restart_the_wait() {
    // Given
    let t0 = Instant::now();
    let mut state = armed(t0);

    // When
    state.pointer_over(Some(titled(1, "Open (o)")), t0 + ms(300));

    // Then
    assert_eq!(state.deadline(), Some(t0 + ms(500)));
}

#[test]
fn another_title_restarts_the_wait_for_its_own_text() {
    // Given
    let t0 = Instant::now();
    let mut state = armed(t0);

    // When
    state.pointer_over(Some(titled(2, "Undo")), t0 + ms(300));

    // Then
    assert_eq!(state.deadline(), Some(t0 + ms(800)));
    assert_eq!(shown_text(&mut state, t0 + ms(800)), Some("Undo".into()));
}

#[test]
fn each_hide_before_the_deadline_leaves_nothing_to_wait_for() {
    let hides: [Hide; 4] = [
        ("leave", TooltipState::left),
        ("press", TooltipState::pressed),
        ("key, wheel, touch or blur", TooltipState::hide),
        ("off the title", |state| {
            state.pointer_over(None, Instant::now());
        }),
    ];
    for (name, hide) in hides {
        // Given
        let t0 = Instant::now();
        let mut state = armed(t0);

        // When
        hide(&mut state);

        // Then
        assert_eq!(state.deadline(), None, "{name}");
        assert_eq!(shown_text(&mut state, t0 + ms(600)), None, "{name}");
    }
}

#[test]
fn a_shown_tooltip_hides_on_each_trigger() {
    let hides: [Hide; 3] = [
        ("leave", TooltipState::left),
        ("press", TooltipState::pressed),
        ("key, wheel, touch or blur", TooltipState::hide),
    ];
    for (name, hide) in hides {
        // Given
        let t0 = Instant::now();
        let mut state = armed(t0);
        assert!(state.shown_at(t0 + ms(500), None).is_some());

        // When
        hide(&mut state);

        // Then
        assert_eq!(shown_text(&mut state, t0 + ms(900)), None, "{name}");
    }
}

#[test]
fn after_a_press_the_title_stays_hidden_until_the_pointer_reaches_another() {
    // Given
    let t0 = Instant::now();
    let mut state = armed(t0);
    state.pressed();
    state.released();

    // When: moves over the pressed title, then another
    state.pointer_over(Some(titled(1, "Open (o)")), t0 + ms(100));
    let still = state.deadline();
    state.pointer_over(Some(titled(2, "Undo")), t0 + ms(200));

    // Then
    assert_eq!(still, None);
    assert_eq!(state.deadline(), Some(t0 + ms(700)));
}

#[test]
fn no_title_arms_while_the_button_is_down() {
    // Given
    let t0 = Instant::now();
    let mut state = TooltipState::default();
    state.pressed();

    // When: a drag across a title
    state.pointer_over(Some(titled(2, "Undo")), t0);

    // Then
    assert_eq!(state.deadline(), None);
}

#[test]
fn after_a_key_hides_it_the_next_move_arms_again() {
    // Given
    let t0 = Instant::now();
    let mut state = armed(t0);
    state.hide();

    // When
    state.pointer_over(Some(titled(1, "Open (o)")), t0 + ms(100));

    // Then
    assert_eq!(state.deadline(), Some(t0 + ms(600)));
}

#[test]
fn keyboard_focus_shows_its_title_after_the_delay() {
    // Given
    let t0 = Instant::now();
    let mut state = TooltipState::default();

    // When
    state.focus(Some(NodeId(3)), Some(titled(3, "Save (Ctrl+S)")), t0);

    // Then
    assert_eq!(shown_text(&mut state, t0 + ms(499)), None);
    let shown = state.shown_at(t0 + ms(500), None).cloned();
    assert_eq!(shown.map(|tip| tip.trigger), Some(TooltipTrigger::Keyboard));
}

#[test]
fn focus_that_stays_does_not_arm_again_and_focus_that_moves_hides_it() {
    // Given
    let t0 = Instant::now();
    let mut state = TooltipState::default();
    state.focus(Some(NodeId(3)), Some(titled(3, "Save")), t0);
    state.hide(); // Esc

    // When / Then: the same focus after another key
    state.focus(Some(NodeId(3)), Some(titled(3, "Save")), t0 + ms(100));
    assert_eq!(state.deadline(), None);

    // When / Then: shown, then blurred
    state.focus(Some(NodeId(4)), Some(titled(4, "Load")), t0 + ms(200));
    assert!(state.shown_at(t0 + ms(700), None).is_some());
    state.focus(None, None, t0 + ms(800));
    assert_eq!(shown_text(&mut state, t0 + ms(900)), None);
}
