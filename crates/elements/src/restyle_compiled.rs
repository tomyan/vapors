//! A keyed element's style from its key's compiled winners
//! (`docs/plans/build-time-styles.md`, row B3a): check 1 and the rules
//! resolved once per key per `Kept`, then each element computed from
//! them (`Cascade::compute_compiled`). The first element of each key in
//! each `Kept` is also computed by the full cascade in every build, and
//! a difference refuses it; in debug builds every element is.

use std::rc::Rc;

use vapors_css::cascade::CompiledRules;
use vapors_dom::{KeyWinners, NodeId, StyleKey, Winners};

use crate::Viewport;
use crate::document::Document;
use crate::error::ElementError;
use crate::style::Counted;
use crate::update::environment;

/// A key's rules in this `Kept`, and whether its first element was
/// verified against the full cascade.
pub(crate) struct CompiledKey {
    rules: Rc<CompiledRules>,
    foreign: bool,
    verified: bool,
}

impl Document {
    /// `node`'s style, made from `key`: from its compiled winners when it
    /// has them, else by the full cascade, counted by which.
    pub(crate) fn compute_keyed(
        &mut self,
        node: NodeId,
        key: &'static StyleKey,
        viewport: Viewport,
    ) -> Result<Rc<Counted>, ElementError> {
        let winners = match key.winners {
            Winners::Compiled(winners) => winners,
            Winners::Unanalysed => {
                return self.counted_full(node, viewport, |c| &mut c.winners_runtime);
            }
            Winners::ComponentTag => {
                return self.counted_full(node, viewport, |c| &mut c.winners_component_tag);
            }
        };
        let (rules, foreign, verified) = self.compiled_key(key, winners)?;
        let style = self.compute_from(node, &rules, viewport)?;
        self.counts.winners_compiled += 1;
        self.counts.winners_foreign += usize::from(foreign);
        if !verified || cfg!(debug_assertions) {
            self.verify(node, key, &style, (viewport, verified))?;
        }
        Ok(style)
    }

    fn counted_full(
        &mut self,
        node: NodeId,
        viewport: Viewport,
        count: fn(&mut crate::UpdateCounts) -> &mut usize,
    ) -> Result<Rc<Counted>, ElementError> {
        *count(&mut self.counts) += 1;
        self.compute_own(node, viewport)
    }

    /// The key's rules in this `Kept`, made and checked on its first
    /// element; with whether another crate's sheet reaches it and
    /// whether its first element was verified.
    fn compiled_key(
        &mut self,
        key: &'static StyleKey,
        winners: &KeyWinners,
    ) -> Result<(Rc<CompiledRules>, bool, bool), ElementError> {
        let address = std::ptr::from_ref(key) as usize;
        let kept = self.kept.as_ref().expect("restyle sets it");
        if let Some(found) = kept.shared.compiled.get(&address) {
            let found = found
                .as_ref()
                .map_err(|why| ElementError::Refused(why.clone()))?;
            return Ok((Rc::clone(&found.rules), found.foreign, found.verified));
        }
        let made = self.make_compiled(key, winners);
        let kept = self.kept.as_mut().expect("restyle sets it");
        let made = kept.shared.compiled.entry(address).or_insert(made);
        let made = made
            .as_ref()
            .map_err(|why| ElementError::Refused(why.clone()))?;
        Ok((Rc::clone(&made.rules), made.foreign, made.verified))
    }

    fn make_compiled(&self, key: &StyleKey, winners: &KeyWinners) -> Result<CompiledKey, String> {
        self.check_compiled_sheets(key)?;
        let rules = self.sheets.compiled_rules(key, winners)?;
        let foreign = self.sheets.foreign_reaches(key, &rules.referenced);
        Ok(CompiledKey {
            rules: Rc::new(rules),
            foreign,
            verified: false,
        })
    }

    fn compute_from(
        &self,
        node: NodeId,
        rules: &CompiledRules,
        viewport: Viewport,
    ) -> Result<Rc<Counted>, ElementError> {
        let kept = self.kept.as_ref().expect("restyle sets it");
        let parent = self.parent(node).map(|p| kept.styles.of(p));
        let env = environment(viewport);
        let style = self
            .sheets
            .compute_compiled(&self.arena, node, parent, (&env, rules))?;
        Ok(kept.styles.alloc(style))
    }

    /// `style` against the full cascade's: on the key's first element a
    /// difference refuses it, in every build; on a later one (debug
    /// builds only) it panics, as the shared-style oracle does.
    fn verify(
        &mut self,
        node: NodeId,
        key: &StyleKey,
        style: &Counted,
        (viewport, verified): (Viewport, bool),
    ) -> Result<(), ElementError> {
        let full = self.compute_own(node, viewport)?;
        let same = **full == **style;
        let differing = || crate::restyle_check::differing_property(Some(style), &full);
        if !verified {
            self.mark_verified(key);
            if !same {
                return Err(ElementError::Refused(format!(
                    "{} (key at {}): its style from its compiled winners differs from the cascade's: \
                     {}; fix: rebuild the crate that compiled it with this vapors-css; reproduce: \
                     mount it after a clean build",
                    self.describe(node),
                    key.site,
                    differing()
                )));
            }
        }
        assert!(
            same,
            "{} (key at {}): its style from its compiled winners differs from the cascade's: {}; \
             vapors_css::analysis and the cascade disagree on this key. Reproduce with the \
             failing test",
            self.describe(node),
            key.site,
            differing()
        );
        Ok(())
    }

    fn mark_verified(&mut self, key: &StyleKey) {
        let address = std::ptr::from_ref(key) as usize;
        let kept = self.kept.as_mut().expect("restyle sets it");
        if let Some(Ok(found)) = kept.shared.compiled.get_mut(&address) {
            found.verified = true;
        }
    }
}
