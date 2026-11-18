//! The code tour template (`plugins/vapors/skills/vapors/assets/code-tour.html`)
//! inlines the `.wisp` grammar; this holds that copy equal to
//! `assets/wisp.tmLanguage.json` beside it.

use std::path::PathBuf;

const OPEN: &str = "<script type=\"application/json\" id=\"wisp-grammar\">\n";
const CLOSE: &str = "\n</script>";

fn asset(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/vapors/skills/vapors/assets")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_code_tour_inlines_the_grammar_file_unchanged() {
    // Given
    let page = asset("code-tour.html");
    let grammar = asset("wisp.tmLanguage.json");

    // When
    let start = page
        .find(OPEN)
        .expect("code-tour.html has no wisp-grammar script")
        + OPEN.len();
    let inlined = &page[start
        ..start
            + page[start..]
                .find(CLOSE)
                .expect("an unclosed wisp-grammar script")];

    // Then
    assert!(
        inlined == grammar.trim_end(),
        "assets/code-tour.html's wisp-grammar script differs from assets/wisp.tmLanguage.json; fix: paste the file's text between the script tags"
    );
}
