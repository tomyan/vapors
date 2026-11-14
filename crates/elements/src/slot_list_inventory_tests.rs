//! Every `SlotList` field is named in its owner's `slot_lists()`, in
//! the same file, so the growth test (`slot_list_document_tests.rs`)
//! sees every kept per-slot list (review of #362, M2).

use std::path::{Path, PathBuf};

/// Fields holding a `SlotList` that no `slot_lists()` names, each with
/// the reason.
const NOT_KEPT: &[(&str, &str, &str)] = &[(
    "layout/tree.rs",
    "boxes",
    "the kept box list lent to Taffy for one layout; `Placed::slot_lists` names it",
)];

/// Every `.rs` file under `dir`, as a path relative to `root`.
fn sources(root: &Path, dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(root, &path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path.strip_prefix(root).expect("under root").to_path_buf());
        }
    }
}

/// The names of the `SlotList` fields declared in `text`: `0` for a
/// tuple struct's.
fn slot_list_fields(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.contains("struct ") && line.contains("(SlotList<") {
                return Some("0".to_owned());
            }
            let (name, kind) = line.split_once(": ")?;
            let named = kind
                .trim_start_matches("crate::slot_list::")
                .starts_with("SlotList<");
            named.then(|| name.rsplit(' ').next().expect("a name").to_owned())
        })
        .collect()
}

/// The text of every `fn slot_lists` body in `text`.
fn inventories(text: &str) -> String {
    text.split("fn slot_lists(")
        .skip(1)
        .map(|rest| rest.split("\n    }\n").next().expect("a body"))
        .collect()
}

/// `file`'s `SlotList` fields that its `slot_lists()` leaves out.
fn missing(file: &str, text: &str) -> Vec<String> {
    let named = inventories(text);
    slot_list_fields(text)
        .into_iter()
        .filter(|field| {
            !NOT_KEPT
                .iter()
                .any(|(f, name, _)| *f == file && name == field)
        })
        .filter(|field| !named.contains(&format!(".{field}.named(")))
        .map(|field| format!("{file}: `{field}`"))
        .collect()
}

#[test]
fn every_slot_list_field_is_named_in_its_owners_slot_lists() {
    // Given
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &root, &mut files);

    // When
    let mut fields = 0;
    let mut left_out = Vec::new();
    for file in &files {
        let name = file.to_str().expect("a UTF-8 path").replace('\\', "/");
        if name == "slot_list_inventory_tests.rs" {
            continue;
        }
        let text = std::fs::read_to_string(root.join(file)).expect("reads");
        fields += slot_list_fields(&text).len();
        left_out.extend(missing(&name, &text));
    }

    // Then
    assert!(
        fields >= 18,
        "found only {fields} SlotList fields: the scan no longer reads them"
    );
    assert!(
        left_out.is_empty(),
        "SlotList fields no slot_lists() names (add `self.<field>.named(..)` to the owner's \
         slot_lists, or to NOT_KEPT with a reason): {left_out:?}"
    );
}
