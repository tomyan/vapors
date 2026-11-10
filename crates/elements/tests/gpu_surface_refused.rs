//! What a `gpu-surface` refuses (docs/gpu-surface.md): no name or an
//! empty one; padding, a border-radius and `display: inline`, each
//! naming the declaration's sheet and line.

mod support;

use support::{document, element, refusals, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;

/// A body holding a `gpu-surface.view` named `name`, with `css` added
/// as `app.css`.
fn page(name: Option<&str>, css: &str) -> Document {
    let mut doc = document();
    let body = doc.body();
    let surface = element(&mut doc, body, "gpu-surface", "view");
    if let Some(name) = name {
        doc.set_attribute(surface, "name", name);
    }
    let css = format!("body {{ margin: 0; }}\n.view {{ width: 40px; height: 30px; }}\n{css}");
    doc.add_stylesheet("app.css", &css).expect("parses");
    doc
}

#[test]
fn a_surface_refuses_what_its_contract_does_not_cover_naming_the_declaration() {
    for (css, says) in [
        (".view { padding: 2px; }", "`padding-top: 2px`"),
        (".view { padding-left: 10%; }", "`padding-left: 10%`"),
        (
            ".view { border-radius: 4px; }",
            "`border-top-left-radius: 4px`",
        ),
        (".view { display: inline; }", "`display: inline`"),
    ] {
        // Given
        let mut doc = page(Some("track"), css);

        // When
        let refused = refusals(&mut doc).join("\n");

        // Then: the declaration on line 3, the element and the property
        assert!(
            refused.contains("app.css:3:")
                && refused.contains("<gpu-surface class=\"view\">")
                && refused.contains(says),
            "{css}: {refused}"
        );
    }
}

#[test]
fn a_surface_with_zero_padding_in_any_unit_is_not_refused() {
    for css in [".view { padding: 0; }", ".view { padding: 0%; }"] {
        // Given
        let mut doc = page(Some("track"), css);

        // When
        let updated = doc.update(viewport(400.0, 300.0));

        // Then
        assert!(updated.is_ok(), "{css}: {:?}", updated.err());
    }
}

#[test]
fn a_surface_without_a_name_or_with_an_empty_one_is_refused_saying_how_to_name_it() {
    for name in [None, Some("")] {
        // Given
        let mut doc = page(name, "");

        // When
        let refused = refusals(&mut doc).join("\n");

        // Then
        assert!(
            refused.contains("name=\"") && refused.contains("ScenePainter::mount"),
            "{name:?}: {refused}"
        );
    }
}
