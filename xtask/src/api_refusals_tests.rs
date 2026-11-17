use super::{ENTRY_CHARS, KEPT, Refusals, escaped};

#[test]
fn a_terminal_escape_sequence_is_printed_escaped() {
    // Given: OSC 52, which would write the lead's clipboard
    let sent = "GET \u{1b}]52;c;cm0gLXJmIH4=\u{7} HTTP/1.1";

    // Then
    let printed = escaped(sent);
    assert!(!printed.chars().any(char::is_control), "{printed:?}");
    assert!(printed.contains("\\u{1b}]52"), "{printed}");
}

#[test]
fn a_long_refusal_is_cut() {
    let printed = escaped(&"a".repeat(10_000));
    assert_eq!(printed.chars().count(), ENTRY_CHARS + 3);
}

#[test]
fn the_log_keeps_the_first_refusals_and_counts_them_all() {
    // Given
    let refusals = Refusals::default();

    // When
    for n in 0..KEPT + 25 {
        refusals.push(&format!("refusal {n}"));
    }

    // Then
    let (kept, count) = refusals.kept();
    assert_eq!((kept.len(), count), (KEPT, (KEPT + 25) as u64));
    assert_eq!(kept[0], "refusal 0");
}
