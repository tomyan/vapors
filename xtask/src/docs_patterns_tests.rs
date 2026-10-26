use super::*;

const FILE: &str = "design/patterns.md";

fn quoted(text: &str) -> (Vec<Quote>, Vec<String>) {
    quotes(FILE, text)
}

fn texts(found: &[Quote]) -> Vec<(usize, &str)> {
    found.iter().map(|q| (q.line, q.text.as_str())).collect()
}

#[test]
fn a_quote_wrapped_across_lines_is_one_quote_from_its_opening_line() {
    // Given
    let text = "# Errors\n\nThe row says \"touche stopped: <why>; fix: reload\n  the page\". Then \"Done\".\n";

    // When
    let (found, problems) = quoted(text);

    // Then
    assert_eq!(problems, Vec::<String>::new());
    assert_eq!(
        texts(&found),
        [
            (3, "touche stopped: <why>; fix: reload the page"),
            (4, "Done")
        ]
    );
}

#[test]
fn quotes_in_code_spans_and_fences_and_single_quotes_are_left_alone() {
    // Given
    let text = "Writes `{\"version\": \"touche\n0.0.0\"}` beside it, as 'Warm it up'.\n\n```\nlet a = \"x\";\n```\n";

    // When
    let (found, problems) = quoted(text);

    // Then
    assert!(
        found.is_empty() && problems.is_empty(),
        "{:?} {problems:?}",
        texts(&found)
    );
}

#[test]
fn a_list_item_is_its_own_block() {
    // Given: the first item's quote is unpaired; the second's is fine
    let text = "- Says \"Offline\n- Then \"Online\".\n";

    // When
    let (found, problems) = quoted(text);

    // Then
    assert_eq!(texts(&found), [(2, "Online")]);
    assert_eq!(
        problems,
        [
            "design/patterns.md:1: a double quote is not closed in its paragraph; fix: close it, or mark an example with single quotes or italics"
        ]
    );
}

#[test]
fn a_quote_with_no_fixed_text_is_refused_naming_its_line() {
    let problems = check(FILE, "Shows \"<count>\" and \"\".\n", &[], "crates/web");
    assert_eq!(
        problems,
        [
            "design/patterns.md:1: \"<count>\" quotes no fixed text; fix: quote the UI text around it, or mark an example with single quotes or italics",
            "design/patterns.md:1: \"\" quotes no fixed text; fix: quote the UI text around it, or mark an example with single quotes or italics",
        ]
    );
}

#[test]
fn placeholders_and_ellipses_split_a_quote_into_trimmed_fixed_pieces() {
    assert_eq!(
        pieces("Voice ended: <why> · <time>… and ...more; fix: reload."),
        ["Voice ended", "and", "more; fix: reload"]
    );
}

#[test]
fn a_composed_message_is_found_when_each_fixed_piece_is_in_some_source() {
    // Given: a message built from a literal, a variable and a constant in another file
    let sources = vec![
        Source::new(
            "start.js",
            "said = \"touché could not start: \" + why + \"; \" + FIX;",
        ),
        Source::new(
            "fix.js",
            "const FIX = 'fix: reload the page, and if it persists clear this site\\'s data';",
        ),
    ];

    // Then
    let quote = "touché could not start: <why>; fix: reload the page, and if it persists clear this site's data";
    assert!(found(&pieces(quote), &sources));
    assert!(!found(
        &pieces("touché could not start: <why>; fix: restart"),
        &sources
    ));
}

#[test]
fn a_quote_missing_from_every_source_is_named_with_its_line_and_fix() {
    // Given
    let text = "Title\n\nShows \"Jump to latest\" and \"Offline\".\n";
    let sources = vec![Source::new("web/index.html", "<span>Offline</span>")];

    // When
    let problems = check(FILE, text, &sources, "crates/web");

    // Then
    assert_eq!(
        problems,
        [
            "design/patterns.md:3: \"Jump to latest\" is not in the page's source (crates/web); fix: quote the text the page shows, or mark an example with single quotes or italics"
        ]
    );
}
