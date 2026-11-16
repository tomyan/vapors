//! A key's sharing answer from its compiled winners (build-time-styles.md,
//! B3b), held in every build against the rules its elements can match
//! (review of #437, M1): a difference refuses the key; a blocking sheet
//! registered with other text is check 1's refusal, not that one (M2).

use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::{CompiledSheet, Dom, KeyWinners, RuleRef, SheetRef, StyleKey, Winners};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const OWN: &str = "cards/card.wisp";
const OTHER: &str = "cards/other.wisp";
const CARD: &str = ".card { color: red; }";
const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// A key of `span.card` whose winners say `OTHER`, read as `other`,
/// blocks sharing; every rule is left to run time.
fn key(other: &'static str) -> &'static StyleKey {
    key_blocked_by(other, &[2])
}

/// [`key`], naming `blocked_by` as its blocking sheets.
fn key_blocked_by(other: &'static str, blocked_by: &'static [u8]) -> &'static StyleKey {
    let sheets = vec![
        (
            SheetRef::UserAgent,
            sheet_hash(vapors_css::user_agent::TEXT),
        ),
        (SheetRef::Scope(OWN), sheet_hash(CARD)),
        (SheetRef::Scope(OTHER), sheet_hash(other)),
    ];
    let count = vapors_css::user_agent::sheet().rules.len();
    let mut conditional: Vec<RuleRef> = (0..count)
        .map(|rule| RuleRef {
            sheet: 0,
            rule: u32::try_from(rule).expect("fits"),
        })
        .collect();
    conditional.extend([RuleRef { sheet: 1, rule: 0 }, RuleRef { sheet: 2, rule: 0 }]);
    let winners = Box::leak(Box::new(KeyWinners {
        sheets: sheets.leak(),
        base: &[],
        conditional: conditional.leak(),
        blocked_by,
        own: Some(1),
    }));
    Box::leak(Box::new(StyleKey {
        site: "cards/card.wisp:3:1",
        scope: Some(OWN),
        tag: "span",
        classes: &["card"],
        toggles: &[],
        attributes: &[],
        style: None,
        winners: Winners::Compiled(winners),
    }))
}

/// Two elements of `key` with OTHER registered as `registered`: the
/// refusal, if any.
fn updated(key: &'static StyleKey, registered: &'static str) -> Result<(), String> {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.verify_as_release();
    doc.add_style(CompiledSheet::for_tests(OWN, CARD));
    doc.add_style(CompiledSheet::for_tests(OTHER, registered));
    let body = doc.body();
    for _ in 0..2 {
        let node = doc.create_keyed_element(key);
        doc.insert_before(body, node, None);
    }
    doc.update(VIEWPORT)
        .map(drop)
        .map_err(|errors| format!("{errors:?}"))
}

#[test]
fn a_compiled_sharing_answer_the_rules_contradict_refuses_the_key_on_the_release_path() {
    // Given: the winners name OTHER as blocking, but its rule does not
    // read siblings, so the rules say the elements may share
    let plain = ":global(.card) { color: green; }";

    // When
    let refused = updated(key(plain), plain).expect_err("refused");

    // Then: both answers, the sheets named, and how to see the compiler's side
    assert!(
        refused.contains("say its elements may not share"),
        "{refused}"
    );
    assert!(refused.contains("say they may"), "{refused}");
    assert!(
        refused.contains("Scope(\\\"cards/other.wisp\\\")"),
        "{refused}"
    );
    assert!(refused.contains("wisp-check -- --styles"), "{refused}");
}

#[test]
fn a_blocking_sheet_registered_with_other_text_is_refused_by_check_1() {
    // Given: the winners read OTHER's sibling rule; the cascade holds
    // other text, with no such rule
    let read = ":global(.card + .card) { color: green; }";

    // When
    let refused = updated(key(read), ":global(.card) { color: green; }").expect_err("refused");

    // Then: the stale sheet named, not a disagreement of the answers
    assert!(refused.contains("hashes"), "{refused}");
    assert!(!refused.contains("may not share"), "{refused}");
}

/// A rule reading siblings, which blocks sharing.
const SIBLINGS: &str = ":global(.card + .card) { color: green; }";

/// The (shared, unshareable) counts of two elements of the blocked key,
/// with the blocking sheet in the cascade or not.
fn shared(blocking_mounted: bool) -> (usize, usize) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style(CompiledSheet::for_tests(OWN, CARD));
    if blocking_mounted {
        doc.add_style(CompiledSheet::for_tests(OTHER, SIBLINGS));
    }
    let (body, key) = (doc.body(), key(SIBLINGS));
    for _ in 0..2 {
        let node = doc.create_keyed_element(key);
        doc.insert_before(body, node, None);
    }
    if let Some(errors) = doc.update(VIEWPORT).err() {
        panic!("{errors:?}");
    }
    let counts = doc.update_counts();
    (counts.styles_shared, counts.styles_unshareable)
}

#[test]
fn a_key_shares_while_the_sheet_its_winners_name_as_blocking_is_not_in_the_cascade() {
    // Then: (shared, unshareable)
    assert_eq!(shared(false), (1, 0));
    assert_eq!(shared(true), (0, 2));
}

#[test]
fn a_blocking_sheet_index_past_the_sheets_refuses_the_key_naming_its_site() {
    // When: the winners name a fourth sheet of three
    let refused = updated(key_blocked_by(SIBLINGS, &[3]), SIBLINGS).expect_err("refused");

    // Then
    assert!(refused.contains("cards/card.wisp:3:1"), "{refused}");
    assert!(
        refused.contains("name sheet 3 of 3 as blocking sharing"),
        "{refused}"
    );
}
