//! The spec's rectangles against the boxes vapors' layout gives at 1280x800.
#![allow(clippy::cast_precision_loss)] // indexes and counts are small
use super::support::*;
use crate::dashboard::*;

#[derive(Clone)]
struct Row(String, &'static str, usize, [Option<f32>; 4]);

fn row(name: String, class: &'static str, nth: usize, want: [Option<f32>; 4]) -> Row {
    Row(name, class, nth, want)
}

fn at(x: f32, y: f32, w: f32, h: f32) -> [Option<f32>; 4] {
    [Some(x), Some(y), Some(w), Some(h)]
}

fn from_x(x: f32) -> [Option<f32>; 4] {
    [Some(x), None, None, None]
}

/// The nav items: the spec's `NAV_*` constants, the icon centred and the label 44 right of the item.
fn nav_rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for i in 0..NAV_ITEMS.len() {
        let y = NAV_TOP + i as f32 * NAV_STRIDE;
        let icon_y = y + (NAV_HEIGHT - NAV_ICON) / 2.0;
        let icon = at(NAV_X + 16.0, icon_y, NAV_ICON, NAV_ICON);
        rows.push(row(
            format!("nav item {i}"),
            "item",
            i,
            at(NAV_X, y, NAV_WIDTH, NAV_HEIGHT),
        ));
        rows.push(row(format!("nav icon {i}"), "icon", i, icon));
        rows.push(row(
            format!("nav label {i}"),
            "nav-label",
            i,
            from_x(NAV_X + 44.0),
        ));
    }
    rows
}

fn sidebar_rows() -> Vec<Row> {
    let user = at(0.0, USER_RULE_Y, SIDEBAR_WIDTH, WINDOW.1 - USER_RULE_Y);
    let avatar = at(20.0, USER_RULE_Y + 20.0, USER_AVATAR, USER_AVATAR);
    vec![
        row(
            "sidebar".into(),
            "sidebar",
            0,
            at(0.0, 0.0, SIDEBAR_WIDTH, WINDOW.1),
        ),
        row(
            "logo".into(),
            "logo",
            0,
            at(20.0, 18.0, LOGO_SIZE, LOGO_SIZE),
        ),
        row("user block".into(), "user", 0, user),
        row("user avatar".into(), "user-avatar", 0, avatar),
        row("user name".into(), "user-name", 0, from_x(64.0)),
        row("user role".into(), "user-role", 0, from_x(64.0)),
    ]
}

fn table() -> Vec<Row> {
    [sidebar_rows(), nav_rows()].concat()
}

#[test]
fn every_box_is_where_the_spec_puts_it() {
    // Given
    let mounted = laid_out();
    let doc = mounted.document.borrow();

    // When
    let misses: Vec<String> = table()
        .iter()
        .filter_map(|Row(name, class, nth, want)| {
            let got = rect(&doc, by_class(&doc, class)[*nth]);
            let off = (0..4).any(|side| want[side].is_some_and(|w| (w - got[side]).abs() > 0.01));
            off.then(|| format!("{name}: want {want:?}, got {got:?}"))
        })
        .collect();

    // Then
    assert!(misses.is_empty(), "{misses:#?}");
}

#[test]
fn the_table_covers_every_box_it_names() {
    assert_eq!(table().len(), 6 + 3 * NAV_ITEMS.len());
}

#[test]
fn the_badge_is_a_20_px_pill_12_px_inside_the_messages_item() {
    // Given
    let mounted = laid_out();
    let doc = mounted.document.borrow();

    // When
    let badges = by_class(&doc, "badge");
    let [x, y, w, h] = rect(&doc, badges[0]);

    // Then
    assert_eq!(badges.len(), 1);
    assert_eq!(text(&doc, badges[0]), BADGE_COUNT.to_string());
    assert_eq!(x + w, NAV_X + NAV_WIDTH - 12.0);
    let item_y = NAV_TOP + BADGE_ITEM as f32 * NAV_STRIDE;
    assert_eq!((y, h), (item_y + 10.0, 20.0));
}

#[test]
fn the_texts_are_the_specs() {
    // Given
    let mounted = laid_out();
    let doc = mounted.document.borrow();
    let texts = |class: &str| -> Vec<String> {
        by_class(&doc, class)
            .into_iter()
            .map(|node| text(&doc, node))
            .collect()
    };

    // Then
    assert_eq!(texts("nav-label"), NAV_ITEMS);
    assert_eq!(texts("user-name"), [USER_NAME]);
    assert_eq!(texts("user-role"), [USER_ROLE]);
    assert_eq!(texts("user-avatar"), [USER_INITIALS]);
}
