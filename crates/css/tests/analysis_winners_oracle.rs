//! `analysis::analyse` against the runtime cascade (`support/
//! winners_oracle.rs` holds the model): for each page, the sheets are
//! added to a `Cascade` in both author orders and every element the
//! key can make (each directive on or off, hovered or not) is
//! computed. A `Constant` winner must be the declaration the cascade
//! took in every situation; an `Undeclared` property must have none;
//! a `Variable` winner must resolve to its base or one of its
//! overriders. A `Constant` winner with a `Constant` value must
//! compute alike under two parents that differ in `color`, `font-size`
//! and `font-weight`.

#[path = "support/winners_oracle.rs"]
mod oracle;
#[path = "support/winners.rs"]
mod support;

use oracle::{allowed, cascade, computed, orders};
use support::{Page, page, winners};
use vapors_css::analysis::{Constancy, Winner};
use vapors_css::property::PropertyId;

/// The pages the hand-written tests use, and a few more shapes.
fn pages() -> Vec<Page> {
    let mut pages = ranked_pages();
    pages.extend(sheet_and_value_pages());
    let mut styled = page(
        "",
        "div.card { color: red }\n.open { width: 1px !important }",
        &[],
    );
    styled.style = Some("color: green; width: 2px");
    pages.push(styled);
    pages
}

/// Rank, specificity and order within the own sheets.
fn ranked_pages() -> Vec<Page> {
    vec![
        page("", ".card { color: red; width: 4px }", &[]),
        page("", ".card { color: red }\n.open { color: blue }", &[]),
        page("", ".open { color: blue }\n.card { color: red }", &[]),
        page(
            "",
            "div.card:hover { color: blue }\n.card { color: red }",
            &[],
        ),
        page(
            "",
            ".card { color: red }\n:where(.open) { color: blue }",
            &[],
        ),
        page(
            "div { color: red !important }",
            ".card { color: blue !important }",
            &[],
        ),
        page("div { color: red }", ".card { color: blue }", &[]),
        page("", ".card { margin-top: 1px }\n.open { margin: 2px }", &[]),
    ]
}

/// The other sheets, and values the cascade resolves against context.
fn sheet_and_value_pages() -> Vec<Page> {
    vec![
        page(
            "",
            ".card { color: blue }",
            &[".card[data-wisp=\"s\"] { color: red }"],
        ),
        page("", ".card { color: blue }", &[".other { color: red }"]),
        page(
            "div { padding-top: 2px }",
            ".card.open { padding-top: 3px }",
            &["div { padding-top: 1px }"],
        ),
        page(
            "",
            ".card { width: 2em; border-top-color: initial; font-weight: bolder }",
            &[],
        ),
        page(
            "",
            ".card:hover { color: red }\n.open:hover { color: blue }",
            &[".card { color: green }"],
        ),
    ]
}

#[test]
fn every_winner_is_what_the_cascade_takes_in_every_situation_and_sheet_order() {
    for (number, page) in pages().iter().enumerate() {
        let analysed = winners(page);
        for order in orders(page) {
            let (cascade, index) = cascade(page, &order);
            for toggles in 0..2u8 {
                for hovered in [false, true] {
                    let style = computed(&cascade, page, toggles, hovered, None);
                    for (winner, id) in analysed.longhands.iter().zip(PropertyId::all()) {
                        let took = style
                            .declared_in(id)
                            .map(|sheet| (sheet, style.declared_at(id).expect("a position")));
                        let allowed = allowed(winner, page, &index, id);
                        assert!(
                            allowed.contains(&took),
                            "page {number}, order {order:?}, toggles {toggles}, hovered {hovered}, {}: the cascade took {took:?}, the analysis allows {allowed:?} from {winner:?}",
                            id.name()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_constant_value_computes_alike_under_two_parents() {
    for (number, page) in pages().iter().enumerate() {
        let analysed = winners(page);
        let (cascade, _) = cascade(page, &orders(page)[0]);
        for toggles in 0..2u8 {
            let plain = computed(&cascade, page, toggles, false, Some(""));
            let loud = computed(
                &cascade,
                page,
                toggles,
                false,
                Some("color: red; font-size: 40px; font-weight: 700"),
            );
            for (winner, id) in analysed.longhands.iter().zip(PropertyId::all()) {
                let constant = matches!(
                    winner,
                    Winner::Constant {
                        value: Constancy::Constant,
                        ..
                    } | Winner::Undeclared {
                        value: Constancy::Constant
                    }
                );
                if constant {
                    assert_eq!(
                        plain.get(id),
                        loud.get(id),
                        "page {number}, toggles {toggles}, {}: judged constant, computed differently under two parents",
                        id.name()
                    );
                }
            }
        }
    }
}
