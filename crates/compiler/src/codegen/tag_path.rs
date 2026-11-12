//! A component tag's path from the crate root: `<crate::a::B>`,
//! `<self::..>` and `<super::..>` from the tag's own module, or a name
//! the script's `use` brings in (`use crate::a::B as C;` makes `<C>`
//! `a::B`). `None` for a path that starts anywhere else: another
//! crate's, or one this module cannot see.

use super::script_uses::UseName;

/// `tag` in the module `module` (its path from the crate root) as a
/// path from the crate root.
pub(crate) fn crate_path(module: &[String], uses: &[UseName], tag: &str) -> Option<Vec<String>> {
    let segments: Vec<String> = tag.split("::").map(str::to_owned).collect();
    let (first, rest) = segments.split_first()?;
    if matches!(first.as_str(), "crate" | "self" | "super") {
        return from_root(module, &segments);
    }
    let imported = uses.iter().find(|name| name.name == *first)?;
    let mut path = imported.path.clone();
    path.extend(rest.iter().cloned());
    from_root(module, &path)
}

/// A path starting `crate`, `self` or `super`s, from the crate root.
fn from_root(module: &[String], path: &[String]) -> Option<Vec<String>> {
    let (first, rest) = path.split_first()?;
    if first == "crate" {
        return Some(rest.to_vec());
    }
    let mut here = module.to_vec();
    let mut rest = path;
    if first == "self" {
        rest = &path[1..];
    } else {
        while let Some(("super", after)) = rest.split_first().map(|(s, a)| (s.as_str(), a)) {
            here.pop()?;
            rest = after;
        }
        if rest.len() == path.len() {
            return None;
        }
    }
    here.extend(rest.iter().cloned());
    Some(here)
}
