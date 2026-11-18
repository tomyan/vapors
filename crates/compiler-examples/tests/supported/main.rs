//! `plugins/vapors/skills/vapors/reference/supported.md`, generated from
//! the tables the compiler, the element tree and the CSS parser refuse
//! by, and held equal to them: the test fails when the committed file is
//! out of date. Regenerate with
//! `UPDATE_SUPPORTED=1 cargo test -p vapors-compiler-examples --test supported`.

mod candidates;
mod css;
mod template;

use std::path::PathBuf;

const REGENERATE: &str =
    "UPDATE_SUPPORTED=1 cargo test -p vapors-compiler-examples --test supported";

const HEADER: &str = "# What vapors supports

Generated from the source tables by `crates/compiler-examples/tests/supported`;
do not edit by hand. Regenerate with
`UPDATE_SUPPORTED=1 cargo test -p vapors-compiler-examples --test supported`.
The unit tests fail when this file is out of date.

Anything not listed is refused with a message naming the file, line and
column. Some refusals happen only when the page runs: the CSS parser
accepts a property's grammar, and the element tree can still refuse a
value it does not lay out yet (see Known refusals).
";

const KNOWN: &str = "## Known refusals

- `white-space: nowrap` parses, but the element tree accepts only
  `white-space: normal`: the page's first update is refused, the message
  goes to `#error`, and nothing else is drawn
  ([#347](https://github.com/tomyan/vapors/issues/347)).
- `display` takes `block`, `inline`, `flex`, `grid` and `none`.
  `inline-block`, `inline-flex` and `inline-grid` are laid out only on a
  flex or grid item, and refused elsewhere when the page runs; a
  `<button>` (`inline-block` in the user-agent sheet) needs a
  `display: flex` or `grid` parent. `display: contents` is refused.
- CSS reaching a `<Mesh>` is not checked at build time: a property the
  mesh does not read (such as `padding`) passes `wisp-check` and the build
  ([#512](https://github.com/tomyan/vapors/issues/512)). Set only the
  properties listed for `mesh` under Scene tags.
";

fn generated() -> String {
    [
        HEADER.to_owned(),
        template::section(),
        css::section(),
        KNOWN.to_owned(),
    ]
    .join("\n")
}

fn committed_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/vapors/skills/vapors/reference/supported.md")
}

#[test]
fn the_supported_reference_matches_the_source_tables() {
    // Given
    let path = committed_path();
    let text = generated();

    // When
    if std::env::var_os("UPDATE_SUPPORTED").is_some() {
        let folder = path.parent().expect("supported.md has a folder");
        std::fs::create_dir_all(folder).expect("create the reference folder");
        std::fs::write(&path, &text).expect("write supported.md");
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();

    // Then
    assert!(
        committed == text,
        "{} is out of date with the source tables; regenerate it with `{REGENERATE}` and commit it",
        path.display()
    );
}

/// `items` as one line of inline code, comma-separated.
pub fn codes<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    items
        .into_iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(", ")
}
