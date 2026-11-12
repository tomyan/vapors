//! What the merge queue remembers between passes, in a JSON file beside
//! its log: heads whose gate posted nothing (skipped for `BACKOFF_SECS`,
//! so one broken head does not take the gate slot every pass), and the
//! last reason logged for each PR (a skip is logged when it changes, not
//! every 2 minutes).

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value, json};

/// How long a head whose gate posted nothing is skipped: 30 min.
pub const BACKOFF_SECS: u64 = 30 * 60;

/// The remembered state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Head to the time (seconds since the epoch) its backoff ends.
    backoff: BTreeMap<String, u64>,
    /// PR number to the last reason logged for it.
    said: BTreeMap<u64, String>,
}

impl State {
    /// The state in `path`; none yet is an empty state.
    ///
    /// # Errors
    ///
    /// A file that cannot be read or is not this shape, naming it.
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text, path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    /// The state in `text`, read from `path`; refused when its shape is not
    /// the queue's.
    fn parse(text: &str, path: &Path) -> Result<Self, String> {
        let refused = |why: &str| {
            format!(
                "{}: not the queue's state ({why}); fix: delete it, the next pass starts afresh",
                path.display()
            )
        };
        let value: Value = serde_json::from_str(text).map_err(|e| refused(&e.to_string()))?;
        Ok(Self {
            backoff: backoff_of(&value).ok_or_else(|| refused("backoff"))?,
            said: said_of(&value).ok_or_else(|| refused("said"))?,
        })
    }

    /// Write the state to `path`.
    ///
    /// # Errors
    ///
    /// It cannot be written, naming the file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let said: Map<String, Value> = self
            .said
            .iter()
            .map(|(k, v)| (k.to_string(), json!(v)))
            .collect();
        let text = json!({ "backoff": self.backoff, "said": said }).to_string();
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Skip `head` for `BACKOFF_SECS` from `now`.
    pub fn back_off(&mut self, head: &str, now: u64) {
        self.backoff.insert(head.to_owned(), now + BACKOFF_SECS);
    }

    /// Drop the backoffs that have ended by `now`, so the file does not
    /// grow by one head per silent gate for the life of the queue.
    pub fn prune(&mut self, now: u64) {
        self.backoff.retain(|_, until| now < *until);
    }

    /// Whether `head` is still backed off at `now`.
    #[must_use]
    pub fn backed_off(&self, head: &str, now: u64) -> bool {
        self.backoff.get(head).is_some_and(|until| now < *until)
    }

    /// Note `reason` for PR `number`; whether it differs from the last.
    pub fn changed(&mut self, number: u64, reason: &str) -> bool {
        self.said.insert(number, reason.to_owned()).as_deref() != Some(reason)
    }
}

/// An object's entries, each key and value read; `None` when one is not.
fn entries<K: Ord, V>(
    value: &Value,
    read: impl Fn(&Value) -> Option<V>,
    key: impl Fn(&str) -> Option<K>,
) -> Option<BTreeMap<K, V>> {
    let Some(object) = value.as_object() else {
        return value.is_null().then(BTreeMap::new);
    };
    object
        .iter()
        .map(|(k, v)| Some((key(k)?, read(v)?)))
        .collect()
}

/// The state's backoffs: head to the second it ends.
fn backoff_of(value: &Value) -> Option<BTreeMap<String, u64>> {
    entries(&value["backoff"], Value::as_u64, |k| Some(k.to_owned()))
}

/// The state's last reasons said: PR to reason.
fn said_of(value: &Value) -> Option<BTreeMap<u64, String>> {
    entries(
        &value["said"],
        |v| v.as_str().map(str::to_owned),
        |k| k.parse().ok(),
    )
}

#[cfg(test)]
#[path = "queue_state_tests.rs"]
mod tests;
