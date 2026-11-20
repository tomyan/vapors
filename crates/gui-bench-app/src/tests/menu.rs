//! The export menu through vapors' test drivers.
#![allow(clippy::cast_precision_loss, clippy::float_cmp)] // counts are small; the extremes are exact
use super::behaviour_top::centre_of;
use super::support::*;
use crate::app::Mounted;
use crate::dashboard::*;
use vapors_elements::Document;

fn open_menu(mounted: &Mounted) {
    click(mounted, centre_of(mounted, "export", 0));
}

fn menu_count(mounted: &Mounted) -> usize {
    by_class(&mounted.document.borrow(), "menu").len()
}

fn background(mounted: &Mounted, nth: usize) -> String {
    let doc = mounted.document.borrow();
    let style = doc
        .computed_style(by_class(&doc, "menu-item")[nth])
        .expect("styled");
    style
        .get_by_name("background-color")
        .expect("a property")
        .to_css()
}

#[test]
fn the_export_button_opens_the_menu_where_the_spec_puts_it() {
    // Given
    let mounted = laid_out();
    assert_eq!(menu_count(&mounted), 0);

    // When
    open_menu(&mounted);

    // Then: the box (border 1 and padding 4 inside its 160) and its items
    let doc = mounted.document.borrow();
    let height = 2.0 * MENU_PADDING + 2.0 + MENU_ITEMS.len() as f32 * MENU_ITEM_HEIGHT;
    assert_eq!(
        rect(&doc, by_class(&doc, "menu")[0]),
        [MENU_X, MENU_Y, MENU_WIDTH, height]
    );
    for (i, label) in MENU_ITEMS.iter().enumerate() {
        let want = [
            MENU_X + 1.0 + MENU_PADDING,
            MENU_Y + 1.0 + MENU_PADDING + i as f32 * MENU_ITEM_HEIGHT,
        ];
        let got = rect(&doc, by_class(&doc, "menu-item")[i]);
        assert_eq!(
            (
                [got[0], got[1], got[3]],
                text(&doc, by_class(&doc, "menu-item")[i])
            ),
            ([want[0], want[1], MENU_ITEM_HEIGHT], (*label).to_string())
        );
    }
}

#[test]
fn the_menu_lies_over_the_content() {
    // Given
    let mounted = laid_out();
    open_menu(&mounted);

    // Then: a point of the menu over the content hits an item, not the card under it
    let doc = mounted.document.borrow();
    let hit = doc.hit([MENU_X + 60.0, MENU_Y + 20.0]).expect("something");
    assert_eq!(doc.attribute(hit, "class"), Some("menu-item"));
}

#[test]
fn an_item_takes_the_hover_colour_under_the_pointer_and_does_nothing_else() {
    // Given
    let mounted = laid_out();
    open_menu(&mounted);
    let (before, active, shown) = (
        background(&mounted, 1),
        mounted.active.get_untracked(),
        mounted.controls.compare.get_untracked(),
    );

    // When
    let point = centre_of(&mounted, "menu-item", 1);
    Document::pointer_moved(&mounted.document, point);
    refresh(&mounted);

    // Then
    let [_, r, g, b] = MENU_HOVER.to_be_bytes();
    assert_eq!(background(&mounted, 1), format!("rgb({r} {g} {b})"));
    assert_ne!(before, background(&mounted, 1));
    assert_eq!(background(&mounted, 0), before);

    // When: the click closes it and changes nothing else
    click(&mounted, point);
    assert_eq!(menu_count(&mounted), 0);
    assert_eq!(
        (
            mounted.active.get_untracked(),
            mounted.controls.compare.get_untracked()
        ),
        (active, shown)
    );
}

#[test]
fn a_click_on_the_button_closes_the_open_menu() {
    // Given
    let mounted = laid_out();
    open_menu(&mounted);

    // When
    open_menu(&mounted);

    // Then
    assert_eq!(menu_count(&mounted), 0);
}
