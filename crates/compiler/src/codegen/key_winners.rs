//! Each template element's compiled winners
//! (`docs/plans/build-time-styles.md`, row B2): a file-level
//! `static WINNERS_{VAR}: vapors_dom::KeyWinners`, made from the styles
//! analysis (`crate::styles`), which the element's `StyleKey` points at.
//! Only the sheets its references index are listed, renumbered from 0,
//! each with the hash of the text the runtime registers for it.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

use vapors_css::analysis::{KeyWinners, Match, Place, SheetId, Winner};
use vapors_css::cascade::{INLINE_SHEET, PRESENTATION_SHEET};

use crate::styles::{Analysis, ElementStyles};

/// A component's analysis, with each of its sheets' hash, by element
/// position.
pub(crate) struct Winners<'a> {
    analysis: &'a Analysis,
    /// `vapors_css::sheet_hash` of each of `analysis.sheets`' text.
    hashes: &'a [u64],
    at: HashMap<(usize, usize), usize>,
}

impl<'a> Winners<'a> {
    pub(crate) fn new(analysis: &'a Analysis, hashes: &'a [u64]) -> Self {
        let at = (analysis.elements.iter().enumerate())
            .map(|(index, element)| ((element.line, element.col), index))
            .collect();
        Self {
            analysis,
            hashes,
            at,
        }
    }

    /// The `KeyWinners` literal of the element at `line`:`col`; `None`
    /// when the analysis holds no element there.
    ///
    /// # Errors
    /// More sheets referenced than a `u8` indexes, naming the count.
    pub(crate) fn literal(&self, line: usize, col: usize) -> Option<Result<String, String>> {
        let element = &self.analysis.elements[*self.at.get(&(line, col))?];
        Some(self.literal_of(element))
    }

    fn literal_of(&self, element: &ElementStyles) -> Result<String, String> {
        let winners = &element.winners;
        let sheets = Sheets::referenced(winners)?;
        let mut out = String::from("vapors_dom::KeyWinners { sheets: &[");
        for &index in &sheets.order {
            let id = sheet_ref(&self.analysis.sheets[index].0);
            let _ = write!(out, "({id}, {}), ", hash_literal(self.hashes[index]));
        }
        out.push_str("], base: &[");
        for (property, place) in base(winners) {
            let _ = write!(
                out,
                "vapors_dom::BaseWinner {{ property: {property}, declaration: {} }}, ",
                sheets.declaration(place)
            );
        }
        out.push_str("], conditional: &[");
        for rule in conditional(winners) {
            let _ = write!(out, "{}, ", sheets.rule(rule.sheet, rule.rule));
        }
        let _ = write!(out, "], shareable: {} }}", winners.shareable);
        Ok(out)
    }
}

/// The sheets a key's references index, in the analysis' order: the
/// user-agent sheet first, always.
struct Sheets {
    order: Vec<usize>,
    renumbered: HashMap<usize, u8>,
}

impl Sheets {
    fn referenced(winners: &KeyWinners) -> Result<Self, String> {
        let mut used = BTreeSet::from([0]);
        used.extend(conditional(winners).map(|rule| rule.sheet));
        used.extend(base(winners).filter_map(|(_, place)| in_rule(place)));
        let order: Vec<usize> = used.into_iter().collect();
        let renumbered = (order.iter().enumerate())
            .map(|(new, &old)| u8::try_from(new).map(|new| (old, new)))
            .collect::<Result<_, _>>()
            .map_err(|_| {
                format!(
                    "the element's winners reference {} sheets; a KeyWinners indexes at most 256",
                    order.len()
                )
            })?;
        Ok(Self { order, renumbered })
    }

    fn rule(&self, sheet: usize, rule: usize) -> String {
        format!(
            "vapors_dom::RuleRef {{ sheet: {}, rule: {rule} }}",
            self.renumbered[&sheet]
        )
    }

    fn declaration(&self, place: Place) -> String {
        match place.sheet {
            INLINE_SHEET => format!("vapors_dom::DeclarationRef::Inline({})", place.declaration),
            PRESENTATION_SHEET => format!(
                "vapors_dom::DeclarationRef::Presentation({})",
                place.declaration
            ),
            sheet => format!(
                "vapors_dom::DeclarationRef::Rule {{ rule: {}, declaration: {} }}",
                self.rule(sheet, place.rule),
                place.declaration
            ),
        }
    }
}

/// `place`'s sheet when it is in a rule, not the literal `style` or a
/// presentation attribute.
fn in_rule(place: Place) -> Option<usize> {
    (place.sheet != INLINE_SHEET && place.sheet != PRESENTATION_SHEET).then_some(place.sheet)
}

/// Each property's always declaration: a constant winner's, or a
/// variable winner's base; longhands by index, then custom properties.
fn base(winners: &KeyWinners) -> impl Iterator<Item = (String, Place)> + '_ {
    let longhands = (winners.longhands.iter().enumerate()).filter_map(|(index, winner)| {
        Some((
            format!("vapors_dom::PropertyRef::Longhand({index})"),
            always(winner)?,
        ))
    });
    let custom = (winners.custom.iter()).filter_map(|(name, winner)| {
        Some((
            format!("vapors_dom::PropertyRef::Custom({name:?})"),
            always(winner)?,
        ))
    });
    longhands.chain(custom)
}

fn always(winner: &Winner) -> Option<Place> {
    match winner {
        Winner::Constant { place, .. } => Some(*place),
        Winner::Variable { base, .. } => *base,
        Winner::Undeclared { .. } => None,
    }
}

/// The candidate rules decided at run time.
fn conditional(
    winners: &KeyWinners,
) -> impl Iterator<Item = &vapors_css::analysis::ClassifiedRule> {
    (winners.rules.iter()).filter(|rule| matches!(rule.verdict, Match::Conditional(_)))
}

/// `id` as the runtime names it: under the crate's name, as the
/// generated `SCOPE` and `SHEET` constants are.
fn sheet_ref(id: &SheetId) -> String {
    match id {
        SheetId::UserAgent => "vapors_dom::SheetRef::UserAgent".to_owned(),
        SheetId::Scope(scope) => format!(
            "vapors_dom::SheetRef::Scope(concat!(env!(\"CARGO_PKG_NAME\"), \"/\", {scope:?}))"
        ),
        SheetId::Global(file) => format!(
            "vapors_dom::SheetRef::Global(concat!(env!(\"CARGO_PKG_NAME\"), \"/\", {file:?}))"
        ),
    }
}

/// `hash` as a hex literal in groups of four digits, as clippy's
/// `unreadable_literal` asks of the generated code.
fn hash_literal(hash: u64) -> String {
    let hex = format!("{hash:016x}");
    let groups: Vec<&str> = (0..4).map(|group| &hex[group * 4..group * 4 + 4]).collect();
    format!("0x{}", groups.join("_"))
}
