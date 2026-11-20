//! The spec's rectangles against the boxes vapors' layout gives at 1280x800.
#![allow(clippy::cast_precision_loss)] // indexes and counts are small
use super::support::*;
use super::{chart_rows, content_rows, filter_rows};
use crate::dashboard::*;

/// A box to find (name, class, nth of that class) and what is pinned of
/// it: `x`, `y`, width, height, each or none.
#[derive(Clone)]
pub(super) struct Row(
    pub String,
    pub &'static str,
    pub usize,
    pub [Option<f32>; 4],
);

pub(super) fn row(name: String, class: &'static str, nth: usize, want: [Option<f32>; 4]) -> Row {
    Row(name, class, nth, want)
}

pub(super) fn at(x: f32, y: f32, w: f32, h: f32) -> [Option<f32>; 4] {
    [Some(x), Some(y), Some(w), Some(h)]
}

pub(super) fn from_x(x: f32) -> [Option<f32>; 4] {
    [Some(x), None, None, None]
}

/// The nav items: the spec's `NAV_*` constants, the icon centred and the label 44 right of the item.
fn nav_rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for i in 0..NAV_ITEMS.len() {
        let y = NAV_TOP + i as f32 * NAV_STRIDE;
        let icon_y = y + (NAV_HEIGHT - NAV_ICON) / 2.0;
        let icon = at(NAV_X + 16.0, icon_y, NAV_ICON, NAV_ICON);
        let item = at(NAV_X, y, NAV_WIDTH, NAV_HEIGHT);
        rows.push(row(format!("nav item {i}"), "item", i, item));
        rows.push(row(format!("nav icon {i}"), "icon", i, icon));
        let label = from_x(NAV_X + 44.0);
        rows.push(row(format!("nav label {i}"), "nav-label", i, label));
    }
    rows
}

fn sidebar_rows() -> Vec<Row> {
    let side = at(0.0, 0.0, SIDEBAR_WIDTH, WINDOW.1);
    let logo = at(20.0, 18.0, LOGO_SIZE, LOGO_SIZE);
    let user = at(0.0, USER_RULE_Y, SIDEBAR_WIDTH, WINDOW.1 - USER_RULE_Y);
    let avatar = at(20.0, USER_RULE_Y + 20.0, USER_AVATAR, USER_AVATAR);
    vec![
        row("sidebar".into(), "sidebar", 0, side),
        row("logo".into(), "logo", 0, logo),
        row("user block".into(), "user", 0, user),
        row("user avatar".into(), "user-avatar", 0, avatar),
        row("user name".into(), "user-name", 0, from_x(64.0)),
        row("user role".into(), "user-role", 0, from_x(64.0)),
    ]
}

fn table() -> Vec<Row> {
    [
        sidebar_rows(),
        nav_rows(),
        content_rows::rows(),
        chart_rows::rows(),
        chart_rows::bars(0, false),
        filter_rows::rows(),
    ]
    .concat()
}

/// Asserts every pinned side of every row, naming the misses.
pub(super) fn assert_rows(doc: &vapors_elements::Document, rows: &[Row]) {
    let misses: Vec<String> = rows
        .iter()
        .filter_map(|Row(name, class, nth, want)| {
            let got = rect(doc, by_class(doc, class)[*nth]);
            let off = (0..4).any(|side| want[side].is_some_and(|w| (w - got[side]).abs() > 0.01));
            off.then(|| format!("{name}: want {want:?}, got {got:?}"))
        })
        .collect();
    assert!(misses.is_empty(), "{misses:#?}");
}

#[test]
fn every_box_is_where_the_spec_puts_it() {
    // Given
    let mounted = laid_out();
    let doc = mounted.document.borrow();

    // Then
    assert_rows(&doc, &table());
}

#[test]
fn the_table_covers_every_box_it_names() {
    // Given
    let mounted = laid_out();
    let doc = mounted.document.borrow();

    // Then: a class the table names exists once per row naming it
    let rows = table();
    let classes: std::collections::BTreeSet<_> = rows.iter().map(|row| row.1).collect();
    for class in classes {
        let named = rows.iter().filter(|row| row.1 == class).count();
        let found = by_class(&doc, class).len();
        assert_eq!(named, found, "{class}");
    }
    let groups = [
        (sidebar_rows().len(), 6),
        (nav_rows().len(), 3 * NAV_ITEMS.len()),
        (content_rows::rows().len(), 6 + KPI_COUNT * (4 + SPARK_BARS)),
        (
            chart_rows::rows().len(),
            4 + SEGMENTS.len() + GRID_LINES.len(),
        ),
        (chart_rows::bars(0, false).len(), CHART_BARS),
        (filter_rows::rows().len(), 6 + 2 * Status::ALL.len()),
    ];
    for (got, want) in groups {
        assert_eq!(got, want);
    }
    assert_eq!(table().len(), groups.iter().map(|g| g.1).sum::<usize>());
}

#[test]
fn the_text_sits_centred_beside_its_icon_and_the_user_name_above_the_role() {
    // Given
    let mounted = laid_out();
    let doc = mounted.document.borrow();
    let mid = |class: &str, nth: usize| {
        let [_, y, _, h] = rect(&doc, by_class(&doc, class)[nth]);
        y + h / 2.0
    };

    // Then: every label shares its item's vertical centre
    for i in 0..NAV_ITEMS.len() {
        assert!(
            (mid("nav-label", i) - mid("item", i)).abs() < 0.5,
            "label {i}"
        );
    }
    let [_, name_y, _, name_h] = rect(&doc, by_class(&doc, "user-name")[0]);
    let [_, role_y, _, _] = rect(&doc, by_class(&doc, "user-role")[0]);
    let [_, avatar_y, _, avatar_h] = rect(&doc, by_class(&doc, "user-avatar")[0]);
    assert!(name_y >= avatar_y && name_y + name_h <= role_y);
    assert!(role_y < avatar_y + avatar_h);
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
