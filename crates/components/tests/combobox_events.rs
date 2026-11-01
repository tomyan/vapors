//! The combobox's default actions on the recording Dom, each after an
//! event an app may cancel: Enter chooses, Escape closes then clears,
//! `ArrowDown` opens.

mod support;

use support::mounted;

#[test]
fn enter_chooses_the_active_option_and_closes() {
    // Given: Alaska active
    let m = mounted();
    m.key("ArrowDown");
    m.key("ArrowDown");

    // When
    m.key("Enter");

    // Then
    assert_eq!(m.value.get(), "Alaska");
    assert_eq!((m.expanded.get(), m.active.get()), (false, None));
}

#[test]
fn a_cancelled_optionchoose_keeps_the_list_and_the_value() {
    // Given
    let m = mounted();
    m.cancel("optionchoose");
    m.key("ArrowDown");

    // When
    m.key("Enter");

    // Then
    assert_eq!(m.value.get(), "");
    assert_eq!((m.expanded.get(), m.active.get()), (true, Some(0)));
}

#[test]
fn a_cancelled_comboboxopen_keeps_the_list_closed() {
    // Given
    let m = mounted();
    m.cancel("comboboxopen");

    // When
    m.key("ArrowDown");

    // Then
    assert_eq!((m.expanded.get(), m.active.get()), (false, None));
}

#[test]
fn escape_closes_an_open_list_then_clears_the_field() {
    // Given: open, with a value
    let m = mounted();
    m.value.set("Al".into());
    m.key("ArrowDown");

    // When
    m.key("Escape");
    let closed = (m.expanded.get(), m.value.get());
    m.key("Escape");

    // Then
    assert_eq!(closed, (false, "Al".to_owned()));
    assert_eq!(m.value.get(), "");
}

#[test]
fn a_cancelled_comboboxdismiss_changes_nothing() {
    // Given
    let m = mounted();
    m.cancel("comboboxdismiss");
    m.key("ArrowDown");

    // When
    m.key("Escape");

    // Then
    assert!(m.expanded.get());
}
