//! The combobox's keys on the recording Dom: the active option moves
//! and wraps, Home/End leave none active and keep the caret's key, and
//! the aria attributes follow the state. Chromium's answers for APG's own
//! example are pinned by the corpus's `combobox.events`.

mod support;

use support::mounted;

#[test]
fn arrow_down_opens_on_the_first_option_and_moves_wrapping() {
    // Given
    let m = mounted();

    // When
    let prevented = m.key("ArrowDown");
    let opened = (m.expanded.get(), m.active.get());
    m.key("ArrowDown");
    m.key("ArrowDown");
    m.key("ArrowDown");

    // Then: open on Alabama; past Arizona back to Alabama
    assert!(prevented);
    assert_eq!(opened, (true, Some(0)));
    assert_eq!(m.active.get(), Some(0));
    assert_eq!(m.attribute("aria-expanded").as_deref(), Some("true"));
    assert_eq!(
        m.attribute("aria-activedescendant").as_deref(),
        Some("cb-option-0")
    );
}

#[test]
fn arrow_up_opens_on_the_last_and_wraps_past_the_first() {
    // Given
    let m = mounted();

    // When
    m.key("ArrowUp");
    let opened = m.active.get();
    m.key("ArrowUp");
    m.key("ArrowUp");
    m.key("ArrowUp");

    // Then
    assert_eq!(opened, Some(2));
    assert_eq!(m.active.get(), Some(2));
}

#[test]
fn home_and_end_leave_no_option_active_and_keep_the_caret_key() {
    // Given: open on Alabama
    let m = mounted();
    m.key("ArrowDown");

    // When
    let prevented = m.key("End");

    // Then: the field's own End still acts
    assert!(!prevented);
    assert_eq!(m.active.get(), None);
    assert!(m.expanded.get());
    assert_eq!(m.attribute("aria-activedescendant").as_deref(), Some(""));
}

#[test]
fn the_active_option_is_selected_and_others_are_not() {
    // Given
    let m = mounted();

    // When
    m.key("ArrowDown");
    m.key("ArrowDown");

    // Then
    let options = m.created("div");
    let selected: Vec<Option<String>> = options
        .iter()
        .filter(|node| m.dom.borrow().attribute(**node, "role").as_deref() == Some("option"))
        .map(|node| m.dom.borrow().attribute(*node, "aria-selected"))
        .collect();
    assert_eq!(
        selected,
        [
            Some("false".into()),
            Some("true".into()),
            Some("false".into())
        ]
    );
    let _ = &m.combobox;
}
