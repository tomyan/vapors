//! GPU tier: `ScenePainter::draw_tooltip` draws the tooltip after the
//! scene, so over a `gpu-surface` and over text; in the colours the
//! root's `--tooltip-*` properties give; placed by `place_tooltip` and
//! inside the window at its edges. `ElementsApp`'s frames are in
//! `gpu_tooltip_app.rs`.
#[path = "support/surface.rs"]
mod support;
#[path = "support/tooltip.rs"]
mod tooltip;

use support::{RED, Rig, SIZE, Solid, document, pixel, update};
use tooltip::{GREEN, TOOLTIP, bounds};
use vapors_app::{Tooltip, TooltipTrigger};
use vapors_dom::Dom;
use vapors_elements::Document;

const MAGENTA: [u8; 4] = [255, 0, 255, 255];

/// A red `gpu-surface` over the whole target, under `TOOLTIP`.
fn over_a_surface(css: &str) -> (Document, Rig) {
    let surface = ".track { position: absolute; left: 0; top: 0; width: 32px; height: 32px; }";
    let mut doc = document(
        &format!("{TOOLTIP} {surface} {css}"),
        &[(None, "gpu-surface", "track")],
    );
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let (solid, _) = Solid::new(RED);
    rig.painter.mount("track", Box::new(solid)).expect("mounts");
    (doc, rig)
}

/// A pointer tooltip showing `text` over the body.
fn tip(doc: &Document, text: &str) -> Tooltip {
    Tooltip {
        node: doc.body(),
        text: text.to_owned(),
        trigger: TooltipTrigger::Pointer,
    }
}

#[test]
fn gpu_the_tooltip_draws_over_a_gpu_surface_at_the_pointer_plus_10_15() {
    // Given
    let (mut doc, mut rig) = over_a_surface("");
    let tooltip = tip(&doc, ".");

    // When
    let rgba = rig
        .draw_tooltip(&mut doc, &tooltip, Some([2.0, 2.0]))
        .expect("draws");

    // Then
    let [left, top, ..] = bounds(&rgba, GREEN).expect("the tooltip's background shows");
    assert_eq!([left, top], [12, 17]);
    assert_eq!([pixel(&rgba, 11, 17), pixel(&rgba, 12, 16)], [RED, RED]);
}

#[test]
fn gpu_the_tooltip_draws_over_the_documents_text() {
    // Given: blue text whose ink covers (12, 18), under the padding
    let text = "p { position: absolute; margin: 0; left: 0; top: 0; font-size: 30px; \
        line-height: 30px; color: #0000ff; }";
    let mut doc = document(&format!("{TOOLTIP} {text}"), &[]);
    let (body, p, ink) = (
        doc.body(),
        doc.create_element("p"),
        doc.create_text("\u{2588}MMM"),
    );
    doc.append(body, p);
    doc.append(p, ink);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let plain = rig.draw(&doc).expect("draws");
    let tooltip = tip(&doc, ".");

    // When
    let rgba = rig
        .draw_tooltip(&mut doc, &tooltip, Some([0.0, 0.0]))
        .expect("draws");

    // Then
    let [_, _, blue, _] = pixel(&plain, 12, 18);
    assert!(blue > 128, "the text's ink is under the tooltip");
    assert_eq!(pixel(&rgba, 12, 18), GREEN);
}

#[test]
fn gpu_the_border_and_text_take_the_colours_set_on_the_root() {
    // Given
    let border = ":root { --tooltip-border-width: 1px; --tooltip-border-color: #ff00ff; }";
    let (mut doc, mut rig) = over_a_surface(border);
    let tooltip = tip(&doc, "M");

    // When
    let rgba = rig
        .draw_tooltip(&mut doc, &tooltip, Some([2.0, 0.0]))
        .expect("draws");

    // Then
    let [left, top, right, bottom] = bounds(&rgba, MAGENTA).expect("the border shows");
    assert_eq!([left, top], [12, 15]);
    assert_ne!(pixel(&rgba, left + 1, top + 1), MAGENTA, "a 1px border");
    let inside = (left + 1..right).flat_map(|x| (top + 1..bottom).map(move |y| (x, y)));
    let inked = inside.filter(|(x, y)| {
        let [r, g, b, _] = pixel(&rgba, *x, *y);
        b > 128 && g < 128 && r < 128
    });
    assert!(inked.count() > 0, "no blue text ink inside the border");
}

#[test]
fn gpu_a_tooltip_at_the_bottom_right_corner_stays_inside_the_window() {
    // Given
    let (mut doc, mut rig) = over_a_surface("");
    let tooltip = tip(&doc, ".");

    // When
    let rgba = rig
        .draw_tooltip(&mut doc, &tooltip, Some([31.0, 31.0]))
        .expect("draws");

    // Then: slid left to the last column, flipped above the hotspot's row
    let [_, _, right, bottom] = bounds(&rgba, GREEN).expect("the tooltip shows");
    assert_eq!([right, bottom], [SIZE - 1, 30]);
}

#[test]
fn gpu_at_scale_2_the_default_radius_rounds_the_corners() {
    // Given: `TOOLTIP` without its radius, so the default 4px (8 device px)
    let style = ":root { --tooltip-background: #00ff00; --tooltip-color: #0000ff; \
        --tooltip-font-size: 8px; --tooltip-padding: 2px; --tooltip-border-width: 0; }";
    let mut doc = document(style, &[]);
    update(&mut doc, 2.0);
    let mut rig = Rig::new();
    let tooltip = tip(&doc, ".");

    // When
    let rgba = rig
        .draw_tooltip(&mut doc, &tooltip, Some([0.0, 0.0]))
        .expect("draws");

    // Then: the corner is cut, the middle of each edge is not
    let [left, top, right, bottom] = bounds(&rgba, GREEN).expect("the tooltip shows");
    let (middle_x, middle_y) = (left.midpoint(right), top.midpoint(bottom));
    assert_eq!(
        [pixel(&rgba, left, middle_y), pixel(&rgba, middle_x, top)],
        [GREEN; 2]
    );
    let corners = [(left, top), (right, top), (left, bottom), (right, bottom)];
    assert!(
        corners.iter().all(|(x, y)| pixel(&rgba, *x, *y) != GREEN),
        "square corners at [{left}, {top}, {right}, {bottom}]"
    );
}

#[test]
fn gpu_text_too_large_for_the_glyph_atlas_is_refused_naming_the_font_size() {
    // Given: whether 900px text fits a 1024px atlas page depends on its
    // glyphs and the scale, so the style check cannot know; "MW" does not.
    // The page is fixed: a default page grows and holds it
    let mut doc = document(":root { --tooltip-font-size: 900px; }", &[]);
    update(&mut doc, 1.0);
    let mut rig = Rig::with_atlas(vapors_text::AtlasConfig::fixed(1024));
    let tooltip = tip(&doc, "MW");

    // When
    let drawn = rig.draw_tooltip(&mut doc, &tooltip, Some([0.0, 0.0]));

    // Then
    let message = drawn.expect_err("the atlas is full").to_string();
    assert!(message.contains("lower `--tooltip-font-size`"), "{message}");
}
