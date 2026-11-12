//! A keyed element in the recording: its kind made from the key, its
//! toggles, and the refusals, recorded as `Call::Refused` with the
//! call changing nothing, as the elements crate returns them from its
//! update.

use std::collections::BTreeMap;

use super::{Call, Kind, RecordingDom};
use crate::{NodeId, StyleKey};

impl RecordingDom {
    /// An element of `key`'s tag with the key's attributes, no
    /// directive on.
    pub(super) fn keyed(key: &'static StyleKey) -> Kind {
        let attributes = (key.attributes().into_iter())
            .map(|(name, value)| (name.to_owned(), value))
            .collect();
        Kind::Element {
            tag: key.tag.into(),
            attributes,
            properties: BTreeMap::new(),
            key: Some((key, 0)),
        }
    }

    /// Turns `toggles` on or off and sets `class` from them.
    ///
    /// # Errors
    /// An element without a key, or a name its key does not list.
    pub(super) fn toggle(&mut self, node: NodeId, toggles: &[(&str, bool)]) -> Result<(), String> {
        let Kind::Element {
            attributes,
            key: Some((key, mask)),
            ..
        } = &mut self.record_mut(node).kind
        else {
            return Err(format!(
                "{node:?} was not created by create_keyed_element, so it has no `class:` directives; create it with create_keyed_element from its template's StyleKey, or set `class` with set_attribute"
            ));
        };
        *mask = key.toggled(*mask, toggles)?;
        attributes.insert("class".into(), key.class_attribute(*mask));
        Ok(())
    }

    /// Records `refusal` as a call on `node`.
    pub(super) fn refuse(&mut self, node: NodeId, refusal: String) {
        self.calls.push(Call::Refused { node, refusal });
    }
}
