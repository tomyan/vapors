//! The names a script's `use` items bring in, with the paths they
//! name, so a component tag can be resolved to the file it names.

/// A name a `use` binds (its alias, or the path's last segment), or
/// `*` for a glob, and the path it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UseName {
    pub name: String,
    pub path: Vec<String>,
}

/// Every name `item` binds.
pub(crate) fn use_names(item: &syn::ItemUse) -> Vec<UseName> {
    let mut names = Vec::new();
    flatten(&item.tree, Vec::new(), &mut names);
    names
}

fn flatten(tree: &syn::UseTree, prefix: Vec<String>, names: &mut Vec<UseName>) {
    let joined = |last: &syn::Ident| {
        let mut path = prefix.clone();
        path.push(last.to_string());
        path
    };
    match tree {
        syn::UseTree::Path(path) => flatten(&path.tree, joined(&path.ident), names),
        syn::UseTree::Name(name) => names.push(UseName {
            name: name.ident.to_string(),
            path: joined(&name.ident),
        }),
        syn::UseTree::Rename(rename) => names.push(UseName {
            name: rename.rename.to_string(),
            path: joined(&rename.ident),
        }),
        syn::UseTree::Glob(_) => names.push(UseName {
            name: "*".to_owned(),
            path: prefix,
        }),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                flatten(item, prefix.clone(), names);
            }
        }
    }
}
