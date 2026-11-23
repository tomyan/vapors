//! The top bar's controls and the export menu at two window widths: the
//! controls keep their distance from the right edge, the menu opens under
//! the button and lies over the content wherever the button is.
#![allow(clippy::cast_precision_loss, clippy::float_cmp)] // counts are small; the boxes are whole
use super::support::*;
use crate::app::{Mounted, mount};
use crate::dashboard::*;
use vapors_elements::Document;
use vapors_scene::{Item, Quad};

/// The spec's window and one narrower, where the metric cards shrink and
/// the fourth card still lies under the menu.
const WIDTHS: [f32; 2] = [WINDOW.0, 1000.0];

/// The gap between the button's bottom and the menu's top (54 - 50).
const MENU_GAP: f32 = MENU_Y - 14.0 - CONTROL_HEIGHT;

fn laid_out_at(width: f32) -> Mounted {
    let mounted = mount(FEED_ROWS);
    refresh_at(&mounted, width);
    mounted
}

fn click_at(mounted: &Mounted, width: f32, point: [f32; 2]) {
    Document::pointer_moved(&mounted.document, point);
    Document::pointer_pressed(&mounted.document, point);
    Document::pointer_released(&mounted.document, point);
    refresh_at(mounted, width);
}

fn box_of(mounted: &Mounted, class: &str) -> Rect {
    let doc = mounted.document.borrow();
    rect(&doc, by_class(&doc, class)[0])
}

fn opened_at(width: f32) -> Mounted {
    let mounted = laid_out_at(width);
    let [x, y, w, h] = box_of(&mounted, "export");
    click_at(&mounted, width, [x + w / 2.0, y + h / 2.0]);
    mounted
}

#[test]
fn the_top_bar_controls_keep_their_distance_from_the_right_edge() {
    for width in WIDTHS {
        // Given
        let mounted = laid_out_at(width);
        let shift = width - WINDOW.0;

        // Then: each control's spec x at 1280, moved with the right edge
        let lefts: Vec<(&str, f32)> = ["search-input", "export", "live-switch", "avatar"]
            .into_iter()
            .map(|class| (class, box_of(&mounted, class)[0]))
            .collect();
        assert_eq!(
            lefts,
            [
                ("search-input", SEARCH_X + shift),
                ("export", EXPORT_X + shift),
                ("live-switch", LIVE_X + shift),
                ("avatar", TOPBAR_AVATAR_X + shift),
            ],
            "at width {width}"
        );
    }
}

#[test]
fn the_menu_opens_under_the_button_at_each_width() {
    for width in WIDTHS {
        // Given
        let mounted = opened_at(width);
        let [x, y, _, h] = box_of(&mounted, "export");

        // Then: its left edge under the button's, MENU_GAP below it
        let height = 2.0 * MENU_PADDING + 2.0 + MENU_ITEMS.len() as f32 * MENU_ITEM_HEIGHT;
        assert_eq!(
            box_of(&mounted, "menu"),
            [x, y + h + MENU_GAP, MENU_WIDTH, height],
            "at width {width}"
        );
    }
}

/// A point of the second item, over the fourth metric card at both widths.
fn over_the_card(menu: Rect) -> [f32; 2] {
    [
        menu[0] + 60.0,
        menu[1] + 1.0 + MENU_PADDING + 1.5 * MENU_ITEM_HEIGHT,
    ]
}

fn contains(rect: Rect, [x, y]: [f32; 2]) -> bool {
    x >= rect[0] && x < rect[0] + rect[2] && y >= rect[1] && y < rect[1] + rect[3]
}

/// The border box of the last painted quad under `point`.
fn top_quad(mounted: &Mounted, point: [f32; 2]) -> Rect {
    let doc = mounted.document.borrow();
    let scene = doc.painted().expect("laid out").scene().to_scene();
    let boxes = scene.items().iter().filter_map(|item| match item {
        Item::Quad(Quad { rect: r, .. }) => {
            Some([r.left, r.top, r.right - r.left, r.bottom - r.top])
        }
        _ => None,
    });
    let found = boxes
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find(|b| contains(*b, point));
    found.expect("a quad under the point")
}

#[test]
fn the_menu_paints_and_takes_hits_over_the_content_at_each_width() {
    for width in WIDTHS {
        // Given: the point lies over the fourth card
        let mounted = opened_at(width);
        let menu = box_of(&mounted, "menu");
        let point = over_the_card(menu);
        let card = {
            let doc = mounted.document.borrow();
            rect(&doc, by_class(&doc, "kpi")[3])
        };
        assert!(
            contains(card, point),
            "at width {width}: {point:?} not over {card:?}"
        );

        // Then: the menu's box is painted last there, and the hit is its item
        assert_eq!(top_quad(&mounted, point), menu, "at width {width}");
        let doc = mounted.document.borrow();
        let hit = doc.hit(point).expect("something");
        assert_eq!(
            doc.attribute(hit, "class"),
            Some("menu-item"),
            "at width {width}"
        );
    }
}
