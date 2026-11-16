//! A keyed element's style from its key's compiled winners
//! (build-time-styles.md, B3a): what they say wins, unless a tested
//! rule beats it; winners that disagree with the cascade are refused on
//! the first element of each conditional state in every build (the
//! release path taken here by `Document::verify_as_release`), and panic
//! on any later element in debug builds; a base entry under a condition
//! and a sheet registered with other text are refused.

use vapors_css::property::PropertyId;
use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::{
    BaseWinner, DeclarationRef, Dom, KeyWinners, PropertyRef, RuleRef, SheetRef, StyleKey, Winners,
};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const OWN: &str = "cards/card.wisp";
/// Rule 0 `.card` red, 1 `.card.x` blue, 2 `.wrap .card` green, 3 an
/// `@media` rule for `span`, which `.card` outranks.
const CARD: &str = ".card { color: red; } .card.x { color: blue; } .wrap .card { color: green; } \
                    @media (min-width: 1px) { span { color: purple; } }";
const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// A key of `span.card` with toggle `x`, whose winners say `color` is
/// won by declaration 0 of rule `rule` of CARD, hashed `hash`, every
/// user-agent rule left to run time so the test names no other winner.
fn key(rule: u32, hash: u64) -> &'static StyleKey {
    key_in(SheetRef::Scope(OWN), rule, hash)
}

/// [`key`] of a component whose sheet is `own`, scoped or global.
fn key_in(own: SheetRef, rule: u32, hash: u64) -> &'static StyleKey {
    let color = PropertyId::from_name("color").expect("a longhand").index();
    let base = vec![BaseWinner {
        property: PropertyRef::Longhand(u16::try_from(color).expect("fits")),
        declaration: DeclarationRef::Rule {
            rule: RuleRef { sheet: 1, rule },
            declaration: 0,
        },
    }];
    let sheets = vec![
        (
            SheetRef::UserAgent,
            sheet_hash(vapors_css::user_agent::TEXT),
        ),
        (own, hash),
    ];
    let winners = Box::leak(Box::new(KeyWinners {
        sheets: sheets.leak(),
        base: base.leak(),
        conditional: user_agent_rules().leak(),
        blocked_by: &[],
        own: Some(1),
    }));
    Box::leak(Box::new(StyleKey {
        site: "cards/card.wisp:3:1",
        scope: matches!(own, SheetRef::Scope(_)).then_some(OWN),
        tag: "span",
        classes: &["card"],
        toggles: &["x"],
        attributes: &[],
        style: None,
        winners: Winners::Compiled(winners),
    }))
}

fn user_agent_rules() -> Vec<RuleRef> {
    let count = vapors_css::user_agent::sheet().rules.len();
    let rule = |rule| RuleRef {
        sheet: 0,
        rule: u32::try_from(rule).expect("fits"),
    };
    (0..count).map(rule).collect()
}

/// One element of the key: `x` toggled on, and inside a `.wrap`.
#[derive(Clone, Copy)]
struct Placed {
    x: bool,
    wrapped: bool,
}

const PLAIN: Placed = Placed {
    x: false,
    wrapped: false,
};
const X: Placed = Placed {
    x: true,
    wrapped: false,
};
const WRAPPED: Placed = Placed {
    x: false,
    wrapped: true,
};

/// A document holding CARD and an element of `key` for each of
/// `placed`, updated, on the release path when `release`: the colours,
/// or the refusal.
fn colours(
    key: &'static StyleKey,
    placed: &[Placed],
    release: bool,
) -> Result<Vec<String>, String> {
    let mut doc = Document::new(FontSystem::bundle_only());
    if release {
        doc.verify_as_release();
    }
    if key.scope.is_some() {
        doc.add_style(OWN, CARD);
    } else {
        doc.add_global_style(OWN, CARD);
    }
    let body = doc.body();
    let nodes: Vec<_> = (placed.iter())
        .map(|&placed| element(&mut doc, body, key, placed))
        .collect();
    if let Some(errors) = doc.update(VIEWPORT).err() {
        return Err(format!("{errors:?}"));
    }
    let colour = |node| {
        let style = doc.computed_style(node).expect("styled");
        style.get_by_name("color").expect("a longhand").to_css()
    };
    Ok(nodes.into_iter().map(colour).collect())
}

fn element(
    doc: &mut Document,
    body: vapors_dom::NodeId,
    key: &'static StyleKey,
    placed: Placed,
) -> vapors_dom::NodeId {
    let parent = if placed.wrapped {
        let wrap = doc.create_element("div");
        doc.set_attribute(wrap, "class", "wrap");
        doc.insert_before(body, wrap, None);
        wrap
    } else {
        body
    };
    let node = doc.create_keyed_element(key);
    doc.toggle_classes(node, &[("x", placed.x)]);
    doc.insert_before(parent, node, None);
    node
}

#[test]
fn an_element_takes_the_colour_its_compiled_winners_name() {
    for release in [false, true] {
        // When
        let found = colours(key(0, sheet_hash(CARD)), &[PLAIN], release);

        // Then
        assert_eq!(
            found,
            Ok(vec!["rgb(255 0 0)".to_owned()]),
            "release path: {release}"
        );
    }
}

#[test]
fn winners_that_disagree_with_the_cascade_refuse_the_first_element_on_either_path() {
    for release in [false, true] {
        // Given: `.card.x`'s blue named as always winning; the element is
        // not `.x`, so the cascade gives red
        let found = colours(key(1, sheet_hash(CARD)), &[PLAIN], release);

        // Then: naming the first longhand that differs (a border colour
        // follows `currentcolor`, so it can come before `color`), and how
        // to see the compiler's side
        let refused = found.expect_err("refused");
        assert!(refused.contains("cards/card.wisp:3:1"), "{refused}");
        assert!(refused.contains("differs from the cascade's"), "{refused}");
        assert!(
            refused.contains(" fresh, ") && refused.contains(" kept"),
            "{refused}"
        );
        assert!(refused.contains("wisp-check -- --styles"), "{refused}");
    }
}

#[test]
fn an_element_in_another_conditional_state_is_verified_and_refused_on_the_release_path() {
    // Given: the first element is `.x`, where `.card.x`'s blue does win;
    // the second is not, a state of its own (review of #435, M1)
    let found = colours(key(1, sheet_hash(CARD)), &[X, PLAIN], true);

    // Then
    assert!(
        found
            .expect_err("refused")
            .contains("differs from the cascade's")
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    should_panic(expected = "differs from the cascade's")
)]
fn winners_that_disagree_on_a_later_element_of_one_state_panic_in_debug_builds() {
    // Given: `.wrap .card`'s green named as always winning; the first
    // element is in a `.wrap`, where it does win; the second, in the same
    // state, is not. Ancestors are not part of the state a release build
    // verifies once (restyle_compiled.rs)
    let found = colours(key(2, sheet_hash(CARD)), &[WRAPPED, PLAIN], false);

    // Then (release builds): both green, as the winners say
    assert_eq!(found, Ok(vec!["rgb(0 128 0)".to_owned(); 2]));
}

#[test]
fn a_base_entry_from_a_rule_under_a_condition_is_refused_naming_the_rule() {
    // When: the `@media` rule named as always winning
    let found = colours(key(3, sheet_hash(CARD)), &[PLAIN], true);

    // Then
    let refused = found.expect_err("refused");
    assert!(refused.contains("a rule under a condition"), "{refused}");
    assert!(refused.contains("`span`"), "{refused}");
}

#[test]
fn a_sheet_registered_with_other_text_refuses_the_element() {
    // When: the winners were read from another `.card` sheet
    let found = colours(key(0, 1), &[PLAIN], true);

    // Then
    let refused = found.expect_err("refused");
    assert!(
        refused.contains("with hash 0x0000_0000_0000_0001"),
        "{refused}"
    );
}

/// A key whose winners say sheet `blocking` (another component's,
/// named "cards/other.wisp") blocks sharing.
fn blocked_key() -> &'static StyleKey {
    let sheets = vec![
        (
            SheetRef::UserAgent,
            sheet_hash(vapors_css::user_agent::TEXT),
        ),
        (SheetRef::Scope(OWN), sheet_hash(CARD)),
        (SheetRef::Scope("cards/other.wisp"), sheet_hash(SIBLINGS)),
    ];
    let mut conditional = user_agent_rules();
    conditional.extend([
        RuleRef { sheet: 1, rule: 0 },
        RuleRef { sheet: 1, rule: 1 },
        RuleRef { sheet: 2, rule: 0 },
    ]);
    let winners = Box::leak(Box::new(KeyWinners {
        sheets: sheets.leak(),
        base: &[],
        conditional: conditional.leak(),
        blocked_by: &[2],
        own: Some(1),
    }));
    Box::leak(Box::new(StyleKey {
        winners: Winners::Compiled(winners),
        ..*key(0, sheet_hash(CARD))
    }))
}

/// A rule reading siblings, which blocks sharing.
const SIBLINGS: &str = ":global(.card + .card) { color: green; }";

/// The share counts of two elements of the blocked key, with the
/// blocking sheet in the cascade or not.
fn shared(blocking_mounted: bool) -> (usize, usize) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style(OWN, CARD);
    if blocking_mounted {
        doc.add_style("cards/other.wisp", SIBLINGS);
    }
    let (body, key) = (doc.body(), blocked_key());
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
fn a_global_keys_winners_that_disagree_with_the_cascade_refuse_its_first_element() {
    // Given: a global component's key, its sheet registered by name
    let found = colours(
        key_in(SheetRef::Global(OWN), 1, sheet_hash(CARD)),
        &[PLAIN],
        true,
    );

    // Then
    let refused = found.expect_err("refused");
    assert!(refused.contains("differs from the cascade's"), "{refused}");
}
