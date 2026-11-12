//! Maps and sets keyed by `NodeId` on the update's hot paths (style,
//! layout, paint), hashed by rustc-hash's `FxHasher` in place of std's
//! `SipHash`, which was about 2 ms of a 21.6 ms edit paint at 3000 rows
//! (docs/perf/baseline-2026-11-10.md, "IU3 3c-2"). Nothing reads their
//! iteration order: where one is iterated, the result is sorted or
//! collected into another map.

use vapors_dom::NodeId;

/// A map by node.
pub(crate) type NodeMap<V> = rustc_hash::FxHashMap<NodeId, V>;

/// A set of nodes.
pub(crate) type NodeSet = rustc_hash::FxHashSet<NodeId>;
