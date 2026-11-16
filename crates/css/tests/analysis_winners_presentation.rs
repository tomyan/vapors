//! The build-time analysis reads a key's literal presentation
//! attributes as the runtime cascade does (`src/cascade/presentation.rs`):
//! a constant winner when no rule declares the property, beaten by an
//! always author rule; a refused attribute fails the analysis.

use vapors_css::analysis::{Constancy, KeyFacts, Place, Sheet, SheetId, Winner, analyse};
use vapors_css::cascade::PRESENTATION_SHEET;
use vapors_css::property::PropertyId;

/// `<path stroke-width=.. fill="none">` under the own sheet `own`.
fn winner(own: &str, width: &'static str, name: &str) -> Result<Winner, String> {
    let attributes: &'static [(&str, &str)] =
        Box::leak(vec![("stroke-width", width), ("fill", "none")].into_boxed_slice());
    let facts = KeyFacts {
        tag: "path",
        scope_attribute: "data-wisp",
        scope: None,
        classes: &[],
        toggles: &[],
        style: None,
        literal_attributes: attributes,
    };
    let (ids, sheets) = (
        [SheetId::UserAgent, SheetId::Scope("s".into())],
        [
            vapors_css::parse("").expect("ua"),
            vapors_css::parse(own).expect("own"),
        ],
    );
    let sheets: Vec<Sheet<'_>> = ids
        .iter()
        .zip(&sheets)
        .map(|(id, rules)| Sheet {
            id,
            rules,
            own: true,
        })
        .collect();
    let winners = analyse(&sheets, &facts, &[])?;
    let id = PropertyId::from_name(name).expect("table");
    Ok(winners.longhands[id.index()].clone())
}

#[test]
fn an_attribute_no_rule_declares_is_a_constant_winner() {
    // stroke-width is the second declaration: attributes are read in
    // the order fill, stroke, stroke-width
    assert_eq!(
        winner("", "1.5", "stroke-width"),
        Ok(Winner::Constant {
            place: Place {
                sheet: PRESENTATION_SHEET,
                rule: 0,
                declaration: 1
            },
            value: Constancy::Constant,
        })
    );
}

#[test]
fn an_always_author_rule_beats_an_attribute() {
    assert_eq!(
        winner("* { fill: red }", "1.5", "fill"),
        Ok(Winner::Constant {
            place: Place {
                sheet: 1,
                rule: 0,
                declaration: 0
            },
            value: Constancy::Constant,
        })
    );
}

#[test]
fn an_attribute_the_cascade_refuses_fails_the_analysis() {
    let refused = winner("", "thick", "stroke-width").expect_err("refused");
    assert!(
        refused.starts_with("the stroke-width attribute of <path> is \"thick\": "),
        "{refused}"
    );
}
