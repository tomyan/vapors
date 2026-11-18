//! What a component's generated file says about its sheet: the `SCOPE`
//! or `SHEET` constant naming it, and the mount's line that registers it.

/// The scope the style is added under and each element is tagged with:
/// the crate and the file, so two crates' `src/panel.wisp` differ.
pub(super) fn scope_constant(scope: &str) -> String {
    format!(
        "/// The scope [`STYLE`] is added under, set on each element as [`SCOPE_ATTRIBUTE`].
pub const SCOPE: &str = {};

",
        in_crate(scope)
    )
}

/// The mount's line that registers the component's sheet, as a
/// `CompiledSheet` with the hash of its text (`[imports.__compiled]`
/// allows the hidden constructor here alone); none without a sheet.
pub(super) fn registration(scoped: bool, global: bool, style: &str) -> String {
    let (method, name) = match (scoped, global) {
        (true, _) => ("add_style", "SCOPE"),
        (false, true) => ("add_global_style", "SHEET"),
        (false, false) => return String::new(),
    };
    let hash = super::key_winners::hash_literal(vapors_css::sheet_hash::sheet_hash(style));
    format!(
        "        dom.borrow_mut().{method}(vapors_elements::__wisp::dom::CompiledSheet::__compiled({name}, STYLE, {hash}));\n"
    )
}

/// A global component's sheet name: its crate and file, as compiled
/// winners name it (`codegen::key_winners`).
pub(super) fn sheet_constant(path: &str) -> String {
    format!(
        "/// The name [`STYLE`] is added under, unscoped, by each mount.
pub const SHEET: &str = {};

",
        in_crate(path)
    )
}

/// A sheet's name as the runtime holds it: `path` after the crate's
/// name. `SCOPE` and every `SheetRef` the winners write are made here,
/// so the two cannot name one sheet differently.
pub(super) fn in_crate(path: &str) -> String {
    format!("concat!(env!(\"CARGO_PKG_NAME\"), \"/\", {path:?})")
}
