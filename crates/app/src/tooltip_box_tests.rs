use vapors_dom::{Dom, NodeId};
use vapors_elements::{BoxRect, Document, Viewport};
use vapors_scene::{Color, Rect};
use vapors_text::FontSystem;

use super::{anchor_of, tooltip_box, wrap_width};
use crate::tooltip_error::TooltipError;
use crate::tooltip_style::TooltipStyle;
use crate::{Anchor, Point, Tooltip, TooltipTrigger};

fn style() -> TooltipStyle {
    TooltipStyle {
        background: Color::from_rgba8(0, 0, 0, 255),
        color: Color::from_rgba8(255, 255, 255, 255),
        border_color: Color::from_rgba8(9, 9, 9, 255),
        font_size: 12.0,
        padding: [3.0, 7.0, 4.0, 7.0],
        border_width: 1.0,
        border_radius: 4.0,
    }
}

/// A 40x20 button at (30, 10) titled "T", updated; and its id.
fn page() -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } button { display: block; margin: 10px 30px; width: 40px; height: 20px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let button = doc.create_element("button");
    doc.set_attribute(button, "title", "T");
    let body = doc.body();
    doc.append(body, button);
    let viewport = Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    (doc, button)
}

fn tip(node: NodeId, trigger: TooltipTrigger) -> Tooltip {
    Tooltip {
        node,
        text: "T".to_owned(),
        trigger,
    }
}

#[test]
fn text_wraps_at_half_the_window_up_to_800_px() {
    assert_eq!(wrap_width(300.0), 150.0);
    assert_eq!(wrap_width(301.0), 151.0);
    assert_eq!(wrap_width(1600.0), 800.0);
    assert_eq!(wrap_width(2560.0), 800.0);
}

#[test]
fn a_pointer_tooltip_anchors_at_the_pointer() {
    // Given
    let (doc, button) = page();
    let painted = doc.painted().expect("updated");

    // When
    let pointer = Some([35.0, 12.0]);
    let anchor = anchor_of(&tip(button, TooltipTrigger::Pointer), pointer, painted);

    // Then
    let hotspot = Point { x: 35.0, y: 12.0 };
    assert_eq!(anchor.expect("anchored"), Anchor::Pointer(hotspot));
}

#[test]
fn a_pointer_tooltip_with_no_pointer_position_is_refused() {
    // Given
    let (doc, button) = page();
    let painted = doc.painted().expect("updated");

    // When
    let anchor = anchor_of(&tip(button, TooltipTrigger::Pointer), None, painted);

    // Then
    assert!(matches!(anchor, Err(TooltipError::NoPointer)));
}

#[test]
fn a_keyboard_tooltip_anchors_at_its_elements_border_box() {
    // Given
    let (doc, button) = page();
    let painted = doc.painted().expect("updated");

    // When
    let anchor = anchor_of(&tip(button, TooltipTrigger::Keyboard), None, painted);

    // Then
    let border_box = BoxRect {
        x: 30.0,
        y: 10.0,
        width: 40.0,
        height: 20.0,
    };
    assert_eq!(anchor.expect("anchored"), Anchor::Keyboard(border_box));
}

#[test]
fn a_keyboard_tooltip_whose_element_has_no_box_is_refused() {
    // Given: an element never appended
    let (mut doc, _) = page();
    let outside = doc.create_element("div");
    let painted = doc.painted().expect("updated");

    // When
    let anchor = anchor_of(&tip(outside, TooltipTrigger::Keyboard), None, painted);

    // Then
    assert!(matches!(anchor, Err(TooltipError::NoBox(node)) if node == outside));
}

#[test]
fn the_box_is_the_text_with_padding_and_border_placed_and_snapped_to_device_pixels() {
    // Given: 20x10 CSS px of text under a pointer at (50, 40), scale 2
    let anchor = Anchor::Pointer(Point { x: 50.0, y: 40.0 });

    // When
    let placed = tooltip_box(&style(), [20.0, 10.0], anchor, [200.0, 100.0], 2.0);

    // Then: at (60, 55) CSS, 36x19 CSS (20 + 14 + 2 by 10 + 7 + 2)
    let placed = placed.expect("placed");
    assert_eq!(placed.quad.rect, Rect::new(120.0, 110.0, 192.0, 148.0));
    assert_eq!(
        (placed.quad.border, placed.quad.radii),
        ([2.0; 4], [8.0; 4])
    );
    assert_eq!(placed.text_origin, [136.0, 118.0]);
}

#[test]
fn the_box_is_clamped_inside_the_window() {
    // Given: a pointer at the bottom-right corner
    let anchor = Anchor::Pointer(Point { x: 199.0, y: 99.0 });

    // When
    let placed = tooltip_box(&style(), [20.0, 10.0], anchor, [200.0, 100.0], 1.0);

    // Then
    let rect = placed.expect("placed").quad.rect;
    assert!(rect.left >= 0.0 && rect.top >= 0.0, "{rect:?}");
    assert!(rect.right <= 200.0 && rect.bottom <= 100.0, "{rect:?}");
}

#[test]
fn a_window_that_cannot_be_placed_in_is_refused() {
    // When
    let anchor = Anchor::Pointer(Point { x: 1.0, y: 1.0 });
    let placed = tooltip_box(&style(), [20.0, 10.0], anchor, [f32::NAN, 100.0], 1.0);

    // Then
    assert!(matches!(placed, Err(TooltipError::Placement(_))));
}
