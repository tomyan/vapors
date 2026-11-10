//! Components vapors ships, written in `.wisp` and compiled by
//! `build.rs`: each file under `src` becomes a module here
//! (`combobox.wisp` is [`combobox::Combobox`]). They depend on the tree
//! only through `vapors-dom` and on `vapors-signals`, so any `Dom` mounts
//! them (the layers check holds this). An app uses one by its path:
//! `<vapors_components::combobox::Combobox ... />`.

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

mod hold_output;
pub use hold_output::hold_into;

/// `prefix` and a number no other call on this process has had: a
/// component's default id, so two left on their defaults never share
/// one (`aria-controls` and `aria-activedescendant` find the first id in
/// the document).
#[must_use]
pub fn unique_id(prefix: &str) -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{prefix}-{n}")
}
