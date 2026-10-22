//! The parsed form of a rule's `@media`, `@container` and `@supports`
//! conditions, and their evaluation.

use super::media::{self, MediaQueryList};
use super::supports::{self, SupportsCondition};
use super::{Environment, media_feature};
use crate::Result;
use crate::ast::Condition;
use crate::selector::Element;

/// Every condition a rule sits under.
#[derive(Clone, Debug, Default)]
pub(crate) struct Conditions {
    media: Vec<MediaQueryList>,
    container: Vec<media::ConditionTree>,
    supports: Vec<SupportsCondition>,
}

impl Conditions {
    /// Parse the conditions; the stylesheet parser calls this so a
    /// malformed query is refused at parse time.
    pub(crate) fn parse(conditions: &[Condition]) -> Result<Self> {
        let mut out = Self::default();
        for c in conditions {
            match c {
                Condition::Media(v) => out.media.push(media::parse_query_list(v)?),
                Condition::Container(v) => out.container.push(media::parse_container(v)?),
                Condition::Supports(v) => out.supports.push(supports::parse(v)?),
            }
        }
        Ok(out)
    }

    /// Whether every condition holds for `element` under `env`.
    pub(crate) fn hold<E: Element>(&self, element: &E, env: &Environment) -> bool {
        self.media.iter().all(|q| q.matches(env))
            && self.supports.iter().all(SupportsCondition::holds)
            && self.container.iter().all(|c| {
                element
                    .container_size()
                    .is_some_and(|(w, h)| c.holds(&media_feature::Sizes::container(w, h, env)))
            })
    }
}
