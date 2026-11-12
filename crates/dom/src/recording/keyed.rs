//! A keyed element in the recording: its kind made from the key, its
//! toggles, and the refusals, recorded as `Call::Refused` with the
//! call changing nothing, as the elements crate returns them from its
//! update. An unread refusal panics when the recording is dropped.

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

    /// Records `refusal` as a call on `node`, unread.
    pub(super) fn refuse(&mut self, node: NodeId, refusal: String) {
        self.unread.push(refusal.clone());
        self.calls.push(Call::Refused { node, refusal });
    }

    /// The refusals recorded since the last call, in order, now read:
    /// a test that expects one takes it here, or the recording panics
    /// when dropped.
    pub fn refusals(&mut self) -> Vec<String> {
        std::mem::take(&mut self.unread)
    }
}

/// An unread refusal fails the test that made it: a test reads the
/// calls it cares about, so a refused one would pass unseen. Not while
/// the thread already panics, so a failing test reports its own
/// failure rather than aborting on a second panic.
impl Drop for RecordingDom {
    fn drop(&mut self) {
        if self.unread.is_empty() || std::thread::panicking() {
            return;
        }
        panic!(
            "RecordingDom dropped with {} refused call(s) no test read: {}; fix the call, or take them with RecordingDom::refusals() where the test expects them",
            self.unread.len(),
            self.unread.join("; ")
        );
    }
}
