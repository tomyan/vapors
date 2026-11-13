//! The analysis as text, for `wisp-check --styles`: per element, its
//! candidate rules by verdict, the declared longhands and custom
//! properties with their winners, and whether its key can share.

use std::fmt::Write;

use vapors_css::analysis::{Constancy, Match, Place, SheetId, Winner};
use vapors_css::cascade::INLINE_SHEET;
use vapors_css::property::PropertyId;

use super::{Analysis, ElementStyles, Resolved};

/// The report, one block per element.
#[must_use]
pub fn report(analysis: &Analysis, path: &str) -> String {
    let mut out = String::new();
    for element in &analysis.elements {
        let _ = writeln!(
            out,
            "{path}:{}:{} <{}>",
            element.line, element.col, element.facts.tag
        );
        rules(&mut out, analysis, element);
        winners(&mut out, analysis, element);
        let _ = writeln!(
            out,
            "  shareable: {}",
            if element.winners.shareable {
                "yes"
            } else {
                "no"
            }
        );
    }
    out
}

fn rules(out: &mut String, analysis: &Analysis, element: &ElementStyles) {
    let counts = element
        .winners
        .rules
        .iter()
        .fold([0; 3], |mut counts, rule| {
            counts[match rule.verdict {
                Match::Always => 0,
                Match::Conditional(_) => 1,
                Match::Never => 2,
            }] += 1;
            counts
        });
    let _ = writeln!(
        out,
        "  rules: {} always, {} conditional, {} never",
        counts[0], counts[1], counts[2]
    );
    for rule in &element.winners.rules {
        let (id, sheet) = &analysis.sheets[rule.sheet];
        let selector = sheet.rules[rule.rule].selector_text();
        let _ = match &rule.verdict {
            Match::Always => writeln!(out, "    always      {}:{} {selector}", name(id), rule.rule),
            Match::Never => writeln!(out, "    never       {}:{} {selector}", name(id), rule.rule),
            Match::Conditional(depends) => writeln!(
                out,
                "    conditional {}:{} {selector} ({depends})",
                name(id),
                rule.rule
            ),
        };
    }
}

fn winners(out: &mut String, analysis: &Analysis, element: &ElementStyles) {
    let declared = element.winners.longhands.iter().zip(PropertyId::all());
    for ((winner, id), resolved) in declared.zip(&element.resolved) {
        if matches!(winner, Winner::Undeclared { .. }) {
            continue;
        }
        let _ = writeln!(
            out,
            "  {}: {}; computed {}",
            id.name(),
            describe(winner, analysis),
            match resolved {
                Resolved::Constant => "constant".to_owned(),
                Resolved::Variable(why) => format!("variable ({why})"),
            }
        );
    }
    for (property, winner) in &element.winners.custom {
        let _ = writeln!(out, "  {property}: {}", describe(winner, analysis));
    }
}

fn describe(winner: &Winner, analysis: &Analysis) -> String {
    match winner {
        Winner::Constant { place, value } => format!(
            "constant winner {} with a {} value",
            at(*place, analysis),
            match value {
                Constancy::Constant => "constant",
                Constancy::Variable => "variable",
                Constancy::FromParent => "parent's",
            }
        ),
        Winner::Variable { base, overriders } => {
            let rules: Vec<String> = (overriders.iter())
                .map(|(sheet, rule)| format!("{}:{rule}", name(&analysis.sheets[*sheet].0)))
                .collect();
            match base {
                Some(place) => format!(
                    "variable winner: {} unless {} matches",
                    at(*place, analysis),
                    rules.join(", ")
                ),
                None => format!("variable winner among {}", rules.join(", ")),
            }
        }
        Winner::Undeclared { .. } => "undeclared".to_owned(),
    }
}

/// `sheet:rule:declaration`, or `style:declaration` for the literal `style`.
fn at(place: Place, analysis: &Analysis) -> String {
    if place.sheet == INLINE_SHEET {
        return format!("style:{}", place.declaration);
    }
    format!(
        "{}:{}:{}",
        name(&analysis.sheets[place.sheet].0),
        place.rule,
        place.declaration
    )
}

fn name(id: &SheetId) -> String {
    match id {
        SheetId::UserAgent => "ua".to_owned(),
        SheetId::Scope(scope) => scope.clone(),
        SheetId::Global(file) => format!("global {file}"),
    }
}
