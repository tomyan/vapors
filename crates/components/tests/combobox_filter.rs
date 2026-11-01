//! The combobox while the app filters its options (it sets `options`):
//! an active option that is gone clears `active` and
//! `aria-activedescendant`, as APG's example resets it while typing, and
//! Enter then closes the list choosing nothing, as APG's does.

mod support;

use support::mounted;

#[test]
fn an_active_option_filtered_away_is_cleared_and_enter_chooses_nothing() {
    // Given: opened on the third option, Arizona
    let combobox = mounted();
    combobox.key("ArrowUp");
    assert_eq!(combobox.active.get(), Some(2));

    // When: the app filters down to one, then Enter
    combobox.options.set(vec!["Alabama".to_owned()]);
    let cleared = (
        combobox.active.get(),
        combobox.attribute("aria-activedescendant"),
    );
    combobox.key("Enter");

    // Then: nothing active nor named; Enter closed the list, choosing nothing
    assert_eq!(cleared, (None, Some(String::new())));
    assert!(!combobox.expanded.get());
    assert_eq!(combobox.value.get(), "");
}

#[test]
fn an_active_option_still_there_at_its_place_stays_active() {
    // Given: Alabama active
    let combobox = mounted();
    combobox.key("ArrowDown");

    // When: the options narrow but keep Alabama first
    combobox
        .options
        .set(vec!["Alabama".to_owned(), "Alaska".to_owned()]);

    // Then
    assert_eq!(combobox.active.get(), Some(0));
    assert_eq!(
        combobox.attribute("aria-activedescendant").as_deref(),
        Some("cb-option-0")
    );
}

#[test]
fn an_active_option_whose_place_now_holds_another_is_cleared() {
    // Given: Alaska active
    let combobox = mounted();
    combobox.key("ArrowDown");
    combobox.key("ArrowDown");

    // When: Alaska is filtered away, and Arizona moves into its place
    combobox
        .options
        .set(vec!["Alabama".to_owned(), "Arizona".to_owned()]);

    // Then
    assert_eq!(combobox.active.get(), None);
}
