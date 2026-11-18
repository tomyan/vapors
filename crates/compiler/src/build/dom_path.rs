//! How generated code reaches `vapors-dom`.

/// The path generated code names the dom crate by. Generated code is
/// written against [`DomPath::Elements`]; the other is a rewrite of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DomPath {
    /// `vapors_elements::__wisp::dom`: the crate needs `vapors-elements`
    /// and not `vapors-dom`. What every page's crate uses.
    #[default]
    Elements,
    /// `vapors_dom`: for a crate below `vapors-elements` (the layers
    /// rule forbids `crates/components` depending on it), which depends
    /// on `vapors-dom` itself.
    VaporsDom,
}

/// What generated code names the dom by, before any rewrite.
const DEFAULT: &str = "vapors_elements::__wisp::dom";

impl DomPath {
    /// `code` naming the dom this way, with a comment under its first
    /// line saying which way.
    pub(crate) fn apply(self, code: &str) -> String {
        let (path, why) = match self {
            Self::Elements => (
                DEFAULT,
                "through vapors-elements (the crate needs no vapors-dom)",
            ),
            Self::VaporsDom => ("vapors_dom", "as vapors_dom (compile_dir_below_elements)"),
        };
        let code = code.replace(DEFAULT, path);
        match code.split_once('\n') {
            Some((first, rest)) => format!("{first}\n// The dom is named {why}.\n{rest}"),
            None => code,
        }
    }
}
