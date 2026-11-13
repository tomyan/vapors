//! Shared by `analysis_winners.rs` and `analysis_winners_values.rs`:
//! a page of sheets as the compiler builds them, analysed for one key.

#![allow(dead_code)] // each test binary uses a subset

use vapors_css::analysis::{Constancy, KeyFacts, Place, Sheet, SheetId, Winner, analyse};
use vapors_css::property::PropertyId;

/// `<div class="card" class:open={..} data-wisp="s">`, optionally with
/// a literal `style`.
pub fn facts(style: Option<&'static str>) -> KeyFacts<'static> {
    KeyFacts {
        tag: "div",
        scope_attribute: "data-wisp",
        scope: Some("s"),
        classes: &["card"],
        toggles: &["open"],
        style,
        literal_attributes: &[],
    }
}

/// The sheets as the compiler builds them: UA, the own sheet scoped
/// as `s`, then every other sheet; `style` is the literal attribute.
pub struct Page {
    pub ids: Vec<SheetId>,
    pub sheets: Vec<vapors_css::ast::Stylesheet>,
    pub style: Option<&'static str>,
}

pub fn page(ua: &str, own: &str, others: &[&str]) -> Page {
    let scoped = vapors_css::scope::scoped(vapors_css::parse(own).expect("own"), "data-wisp", "s");
    let mut ids = vec![SheetId::UserAgent, SheetId::Scope("s".into())];
    let mut sheets = vec![vapors_css::parse(ua).expect("ua"), scoped];
    for (index, other) in others.iter().enumerate() {
        ids.push(SheetId::Scope(format!("other{index}")));
        sheets.push(vapors_css::parse(other).expect("other"));
    }
    Page {
        ids,
        sheets,
        style: None,
    }
}

pub fn winners(page: &Page) -> vapors_css::analysis::KeyWinners {
    let sheets: Vec<Sheet<'_>> = (page.ids.iter().zip(&page.sheets).enumerate())
        .map(|(index, (id, rules))| Sheet {
            id,
            rules,
            own: index < 2,
        })
        .collect();
    let inline = page
        .style
        .map(|style| vapors_css::parse_inline(style).expect("inline"))
        .unwrap_or_default();
    analyse(&sheets, &facts(page.style), &inline).unwrap_or_else(|e| panic!("{e}"))
}

pub fn longhand(page: &Page, name: &str) -> Winner {
    winners(page).longhands[PropertyId::from_name(name).expect("longhand").index()].clone()
}

pub fn constant(sheet: usize, rule: usize, declaration: usize, value: Constancy) -> Winner {
    Winner::Constant {
        place: Place {
            sheet,
            rule,
            declaration,
        },
        value,
    }
}
