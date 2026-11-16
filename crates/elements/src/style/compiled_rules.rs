//! A key's compiled winners as rules of this cascade
//! (`docs/plans/build-time-styles.md`, section 2, "Used at run time"):
//! each sheet they reference found in the cascade by its name, each rule
//! by its place in its sheet.

use vapors_css::cascade::{CompiledRules, ComputedStyle, Environment};
use vapors_dom::{DeclarationRef, KeyWinners, RuleRef, SheetRef, StyleKey};

use super::Sheets;
use crate::arena::Arena;
use crate::css_element::ElementRef;
use crate::error::ElementError;
use vapors_dom::NodeId;

impl Sheets {
    /// `winners` as indices into this cascade. A sheet they reference
    /// that is absent has no rules here and is left out; a base
    /// declaration of the literal `style` or a presentation attribute
    /// needs no entry, as the cascade enters those for every element.
    ///
    /// # Errors
    /// A rule or declaration a present sheet does not have, naming the
    /// key's site: winners read from other text, which check 1 refuses
    /// first unless the hash itself was forged.
    pub(crate) fn compiled_rules(
        &self,
        key: &StyleKey,
        winners: &KeyWinners,
    ) -> Result<CompiledRules, String> {
        let sheets: Vec<Option<usize>> = (winners.sheets.iter())
            .map(|&(sheet, _)| self.index_of(sheet))
            .collect();
        let mut compiled = CompiledRules {
            base: self.base_entries(key, winners, &sheets)?,
            ..CompiledRules::default()
        };
        for &rule in winners.conditional {
            compiled.tested.extend(self.rule_of(key, &sheets, rule)?);
        }
        compiled.referenced = sheets.into_iter().flatten().collect();
        compiled.referenced.sort_unstable();
        Ok(compiled)
    }

    /// The (rule, declaration) of each base winner in a rule of a present
    /// sheet, once each.
    fn base_entries(
        &self,
        key: &StyleKey,
        winners: &KeyWinners,
        sheets: &[Option<usize>],
    ) -> Result<Vec<(usize, usize)>, String> {
        let mut entries = Vec::new();
        for winner in winners.base {
            let DeclarationRef::Rule { rule, declaration } = winner.declaration else {
                continue;
            };
            let Some(index) = self.rule_of(key, sheets, rule)? else {
                continue;
            };
            let entry = (index, declaration as usize);
            self.base_entry_holds(key, entry)?;
            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// `sheet`'s place in the cascade, `None` when absent: from the one
    /// table, by kind and name, that check 1 reads too (`Sheets::held`).
    pub(crate) fn index_of(&self, sheet: SheetRef) -> Option<usize> {
        self.held(sheet).map(|held| held.index)
    }

    fn rule_of(
        &self,
        key: &StyleKey,
        sheets: &[Option<usize>],
        rule: RuleRef,
    ) -> Result<Option<usize>, String> {
        let Some(sheet) = sheets.get(usize::from(rule.sheet)).ok_or_else(|| {
            format!(
                "{}: its compiled winners name sheet {} of {}",
                key.site,
                rule.sheet,
                sheets.len()
            )
        })?
        else {
            return Ok(None);
        };
        let index = self.cascade.rule_index(*sheet, rule.rule as usize);
        index.map(Some).ok_or_else(|| {
            format!(
                "{}: its compiled winners name rule {} of {}, which has no such rule; fix: \
                 rebuild the crate that compiled it; reproduce: mount it after a clean build",
                key.site, rule.rule, self.names[*sheet]
            )
        })
    }

    /// Whether `(rule, declaration)` may be a base entry: the declaration
    /// exists, and its rule sits under no `@media`, `@supports` or
    /// `@container` condition, which base entries are entered without
    /// testing (the analysis judges such a rule conditional).
    fn base_entry_holds(
        &self,
        key: &StyleKey,
        (rule, declaration): (usize, usize),
    ) -> Result<(), String> {
        let found = self.cascade.rule_at(rule);
        let refuse = |why: String| {
            Err(format!(
                "{}: its compiled winners name {why}; fix: rebuild the crate that compiled it with \
                 this vapors-css, else report the analysis; reproduce: `cargo run -p \
                 vapors-compiler --bin wisp-check -- --styles <its .wisp>` prints them",
                key.site
            ))
        };
        match found {
            Some((rule, _)) if declaration >= rule.declarations.len() => refuse(format!(
                "declaration {declaration} of a rule that has fewer"
            )),
            Some((rule, true)) => refuse(format!(
                "a rule under a condition as always winning: `{}`, whose condition only the \
                 element's environment decides",
                vapors_css::serialize::component_values(&rule.prelude).trim()
            )),
            Some(_) => Ok(()),
            None => refuse(format!("rule {rule}, which the cascade does not have")),
        }
    }

    /// Whether a sheet the key does not reference has a candidate rule
    /// for it: another crate's (check 2), counted in `winners_foreign`.
    pub(crate) fn foreign_reaches(&self, key: &StyleKey, referenced: &[usize]) -> bool {
        let classes: Vec<&str> = key.classes.iter().chain(key.toggles).copied().collect();
        self.cascade.foreign_reaches(key.tag, &classes, referenced)
    }

    /// The computed style of `node` from `compiled`.
    pub(crate) fn compute_compiled(
        &self,
        arena: &Arena,
        node: NodeId,
        parent: Option<&ComputedStyle>,
        (env, compiled): (&Environment, &CompiledRules),
    ) -> Result<ComputedStyle, ElementError> {
        self.cascade
            .compute_compiled(&ElementRef::new(arena, node), parent, env, compiled)
            .map_err(|error| ElementError::Css {
                sheet: self.names.join(" or "),
                error,
            })
    }
}
