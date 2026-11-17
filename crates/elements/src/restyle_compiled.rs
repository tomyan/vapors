//! A keyed element's style from its key's compiled winners
//! (`docs/plans/build-time-styles.md`, row B3a): check 1 and the rules
//! resolved once per key per `Kept`, then each element computed from
//! them (`Cascade::compute_compiled`). In every build, the first element
//! of each key in each conditional state (its states, toggles and the
//! attributes a sheet selects: the share key's inputs but its parent) is
//! also computed by the full cascade, and a difference refuses that key
//! for the rest of the `Kept`; debug builds compare every element.
//!
//! The gap a release build leaves: the state holds nothing of the
//! element's ancestors or siblings, so a winner that depends on them (a
//! descendant selector such as `.wrap .card`, a sibling one such as
//! `.a + .card`, or `:has()`) wrongly entered as always winning is
//! caught only when the first element of its state is one it does not
//! hold for; a later element of an already verified state is compared
//! in debug builds only. The plan's B3a row states the same.

use std::rc::Rc;

use vapors_css::cascade::CompiledRules;
use vapors_dom::{KeyWinners, NodeId, StyleKey, Winners};

use crate::Viewport;
use crate::document::Document;
use crate::error::ElementError;
use crate::style::{Counted, ShareKey};
use crate::update::environment;

/// A key's rules in this `Kept`.
pub(crate) struct CompiledKey {
    rules: Rc<CompiledRules>,
    foreign: bool,
}

/// A key's rules and whether another crate's sheet reaches it, or its
/// refusal.
fn found_rules(
    found: &Result<CompiledKey, String>,
) -> Result<(Rc<CompiledRules>, bool), ElementError> {
    let found = found
        .as_ref()
        .map_err(|why| ElementError::Refused(why.clone()))?;
    Ok((Rc::clone(&found.rules), found.foreign))
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
        let (rules, foreign) = self.compiled_key(key, winners)?;
        let style = self.compute_from(node, &rules, viewport)?;
        self.counts.winners_compiled += 1;
        self.counts.winners_foreign += usize::from(foreign);
        let first = self.first_in_state(node, key);
        if first || self.compare_every_element {
            self.verify(node, key, &style, (viewport, first))?;
        }
        Ok(style)
    }

    /// The test's switch to the release path: only the first element of
    /// each key in each state is compared, as in a release build.
    /// RELAXED: no test tier builds release, so the release path is
    /// reached by this switch in a debug build; retired when a tier runs
    /// the elements tests on the release profile.
    #[doc(hidden)]
    pub fn verify_as_release(&mut self) {
        self.compare_every_element = false;
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
    /// element, with whether another crate's sheet reaches it; or the
    /// key's refusal, for every element of it.
    pub(crate) fn compiled_key(
        &mut self,
        key: &'static StyleKey,
        winners: &KeyWinners,
    ) -> Result<(Rc<CompiledRules>, bool), ElementError> {
        let address = std::ptr::from_ref(key) as usize;
        let kept = self.kept.as_ref().expect("restyle sets it");
        if let Some(found) = kept.shared.compiled.get(&address) {
            return found_rules(found);
        }
        let made = self.make_compiled(key, winners);
        let found = found_rules(&made);
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.shared.compiled.insert(address, made);
        found
    }

    fn make_compiled(&self, key: &StyleKey, winners: &KeyWinners) -> Result<CompiledKey, String> {
        self.check_compiled_sheets(key)?;
        let rules = self.sheets.compiled_rules(key, winners)?;
        let foreign = self.sheets.foreign_reaches(key, &rules.referenced);
        Ok(CompiledKey {
            rules: Rc::new(rules),
            foreign,
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

    /// Whether `node` is the first element of `key` in its conditional
    /// state this `Kept`; it is noted as seen.
    fn first_in_state(&mut self, node: NodeId, key: &'static StyleKey) -> bool {
        let state = ShareKey {
            parent: 0,
            ..self.share_key(node, key)
        };
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.shared.verified.insert(state)
    }

    /// `style` against the full cascade's: on the first element of a
    /// state a difference refuses the key for the rest of the `Kept`, in
    /// every build; on a later one (debug builds) it panics, as the
    /// shared-style oracle does.
    fn verify(
        &mut self,
        node: NodeId,
        key: &StyleKey,
        style: &Counted,
        (viewport, first): (Viewport, bool),
    ) -> Result<(), ElementError> {
        let full = self.compute_own(node, viewport)?;
        if **full == **style {
            return Ok(());
        }
        let why = format!(
            "{} (key at {}): its style from its compiled winners differs from the cascade's: {}; \
             vapors_css::analysis and the cascade disagree on this key, or it was compiled with \
             another vapors-css; fix: rebuild its crate with this vapors-css, else report the \
             analysis; reproduce: `cargo run -p vapors-compiler --bin wisp-check -- --styles \
             <its .wisp>` prints the compiler's winners for the site",
            self.describe(node),
            key.site,
            crate::restyle_check::differing_property(Some(style), &full)
        );
        assert!(first, "{why}");
        self.refuse_key(key, &why);
        Err(ElementError::Refused(why))
    }

    /// Refuses every later element of `key` in this `Kept`.
    pub(crate) fn refuse_key(&mut self, key: &StyleKey, why: &str) {
        let address = std::ptr::from_ref(key) as usize;
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.shared.compiled.insert(address, Err(why.to_owned()));
    }
}
