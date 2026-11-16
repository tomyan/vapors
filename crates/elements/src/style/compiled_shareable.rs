//! Whether a key's elements may share one computed style, by its
//! compiled winners (`docs/plans/build-time-styles.md`, row B3b), and by
//! the rules its elements can match, which every build holds it against
//! (`Document::shareable`).

use vapors_css::cascade::unshareable;
use vapors_dom::{KeyWinners, StyleKey};

use super::Sheets;

impl Sheets {
    /// Whether `key`'s elements may share by its compiled winners: no
    /// sheet in the cascade that they name as blocking, and no rule of a
    /// sheet they do not reference (another crate's), blocks it.
    pub(crate) fn compiled_shareable(
        &self,
        key: &StyleKey,
        winners: &KeyWinners,
    ) -> Result<bool, String> {
        let mut referenced: Vec<usize> = (winners.sheets.iter())
            .filter_map(|&(sheet, _)| self.index_of(sheet))
            .collect();
        referenced.sort_unstable();
        let mut blocked = false;
        for &at in winners.blocked_by {
            let (sheet, _) = winners.sheets.get(usize::from(at)).ok_or_else(|| {
                format!(
                    "{}: its compiled winners name sheet {at} of {} as blocking sharing; fix: \
                     rebuild the crate that compiled it; reproduce: `cargo run -p \
                     vapors-compiler --bin wisp-check -- --styles <its .wisp>`",
                    key.site,
                    winners.sheets.len()
                )
            })?;
            blocked |= self.index_of(*sheet).is_some();
        }
        let classes = classes_of(key);
        let mut foreign = self
            .cascade
            .foreign_rules_for_key(key.tag, &classes, &referenced);
        Ok(!blocked && !foreign.any(|rule| unshareable(rule).is_some()))
    }

    /// Why `key`'s compiled sharing answer `compiled` and the matched
    /// rules' differ: both answers, the sheets the winners name as
    /// blocking, and the first rule that blocks sharing.
    pub(crate) fn shareable_disagreement(
        &self,
        key: &StyleKey,
        winners: &KeyWinners,
        compiled: bool,
    ) -> String {
        let named: Vec<String> = (winners.blocked_by.iter())
            .filter_map(|&at| winners.sheets.get(usize::from(at)))
            .map(|(sheet, _)| format!("{sheet:?}"))
            .collect();
        let classes = classes_of(key);
        let first = (self.rules_for_key(key.tag, &classes))
            .find(|rule| unshareable(rule).is_some())
            .map_or_else(
                || "none".to_owned(),
                |rule| {
                    format!(
                        "`{}`",
                        vapors_css::serialize::component_values(&rule.prelude).trim()
                    )
                },
            );
        format!(
            "{}: its compiled winners say its elements {} share, the rules they can match say \
             they {} (blocking sheets named: [{}]; first blocking rule in the cascade: {first}); \
             fix: rebuild the crate that compiled it, else report the analysis; reproduce: \
             `cargo run -p vapors-compiler --bin wisp-check -- --styles <its .wisp>` prints its \
             blocking sheets",
            key.site,
            if compiled { "may" } else { "may not" },
            if compiled { "may not" } else { "may" },
            named.join(", ")
        )
    }

    /// Whether `key`'s elements may share by the rules any of them can
    /// match.
    pub(crate) fn runtime_shareable(&self, key: &StyleKey) -> bool {
        let classes = classes_of(key);
        !self
            .rules_for_key(key.tag, &classes)
            .any(|rule| unshareable(rule).is_some())
    }
}

/// A key's literal classes and toggles: every class its elements can have.
pub(super) fn classes_of(key: &StyleKey) -> Vec<&'static str> {
    key.classes.iter().chain(key.toggles).copied().collect()
}
