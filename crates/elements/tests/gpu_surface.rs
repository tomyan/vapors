//! `gpu-surface` in the scene (docs/gpu-surface.md): its renderer's
//! content over its content box, after its own background and before
//! its children, under its ancestors' clip; nothing when hidden or
//! empty; and what it refuses.

mod support;

use support::{document, element, refusals, viewport};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_scene::{Item, Rect};

/// A body holding a `div.panel` holding a `gpu-surface.view` named
/// "track", with `css` added.
fn page(css: &str) -> Document {
    let mut doc = document();
    let body = doc.body();
    let panel = element(&mut doc, body, "div", "panel");
    let surface = element(&mut doc, panel, "gpu-surface", "view");
    doc.set_attribute(surface, "name", "track");
    element(&mut doc, surface, "div", "overlay");
    let css = format!("body {{ margin: 0; }} {css}");
    doc.add_stylesheet("app.css", &css).expect("parses");
    doc
}

/// A surface item's name, rect and clip.
type Drawn = (String, Rect, Option<Rect>);

/// The kinds of the scene's items, in order, with each surface's name,
/// rect and clip at `scale`.
fn painted(doc: &mut Document, scale: f32) -> (Vec<&'static str>, Vec<Drawn>) {
    let painted = doc
        .update(Viewport {
            scale,
            ..viewport(200.0, 200.0)
        })
        .expect("updates");
    let scene = painted.scene();
    let kinds = scene.items().iter().map(kind).collect();
    let surfaces = scene
        .items()
        .iter()
        .zip(scene.clips())
        .filter_map(|(item, clip)| match item {
            Item::Surface { surface, rect } => {
                Some((painted.surface(*surface).to_owned(), *rect, *clip))
            }
            _ => None,
        })
        .collect();
    (kinds, surfaces)
}

fn kind(item: &Item) -> &'static str {
    match item {
        Item::Quad(_) => "quad",
        Item::Surface { .. } => "surface",
        Item::Image { .. } | Item::Text { .. } | Item::Shadow(_) => "other",
    }
}

const SIZED: &str = ".view { width: 40px; height: 30px; margin-left: 5px; \
    border: 1px solid black; background-color: #102030; } \
    .overlay { width: 10px; height: 10px; background-color: #ffffff; }";

#[test]
fn a_surface_draws_over_its_content_box_between_its_background_and_its_children() {
    // Given
    let mut doc = page(SIZED);

    // When
    let (kinds, surfaces) = painted(&mut doc, 2.0);

    // Then: inside the 1 px border, at 2x; after the box, before the overlay
    assert_eq!(kinds, ["quad", "surface", "quad"]);
    assert_eq!(
        surfaces,
        [("track".to_owned(), Rect::new(12.0, 2.0, 92.0, 62.0), None)]
    );
}

#[test]
fn a_surface_under_a_clipping_panel_carries_the_panels_clip() {
    // Given: the panel is 20 px tall and clips
    let css = format!("{SIZED} .panel {{ height: 20px; overflow: hidden; }}");
    let mut doc = page(&css);

    // When
    let (_, surfaces) = painted(&mut doc, 1.0);

    // Then
    assert_eq!(surfaces[0].2, Some(Rect::new(0.0, 0.0, 200.0, 20.0)));
}

#[test]
fn a_hidden_or_empty_surface_paints_no_surface() {
    for css in [
        format!("{SIZED} .view {{ visibility: hidden; }}"),
        format!("{SIZED} .view {{ width: 0; }}"),
        format!("{SIZED} .view {{ height: 0; }}"),
        format!("{SIZED} .view {{ display: none; }}"),
    ] {
        // Given
        let mut doc = page(&css);

        // When
        let (_, surfaces) = painted(&mut doc, 1.0);

        // Then
        assert_eq!(surfaces, [], "{css}");
    }
}

#[test]
fn a_surface_refuses_what_its_contract_does_not_cover() {
    // an inline one without the border and margin, which inline boxes
    // refuse before a surface is checked
    for (css, says) in [
        (format!("{SIZED} .view {{ padding: 2px; }}"), "padding"),
        (
            format!("{SIZED} .view {{ border-radius: 4px; }}"),
            "border-radius",
        ),
        (
            ".view { display: inline; }".to_owned(),
            "a gpu-surface is inline",
        ),
    ] {
        // Given
        let mut doc = page(&css);

        // When
        let refused = refusals(&mut doc).join("\n");

        // Then
        assert!(
            refused.contains("gpu-surface") && refused.contains(says),
            "{css}: {refused}"
        );
    }
}

#[test]
fn a_surface_without_a_name_is_refused_saying_how_to_name_it() {
    // Given
    let mut doc = page(SIZED);
    let body = doc.body();
    element(&mut doc, body, "gpu-surface", "bare");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(refused.contains("name=\""), "{refused}");
}
