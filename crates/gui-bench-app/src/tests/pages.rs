//! The pages: the nav click switches them, the messages feed lays out
//! as the spec says, the other items show the empty page.
#![allow(clippy::cast_precision_loss)] // indexes and counts are small
use super::behaviour_top::{centre_of, texts};
use super::support::*;
use crate::dashboard::*;

fn nav(mounted: &crate::app::Mounted, item: usize) {
    click(mounted, centre_of(mounted, "item", item));
}

fn count(mounted: &crate::app::Mounted, class: &str) -> usize {
    by_class(&mounted.document.borrow(), class).len()
}

#[test]
fn the_nav_click_switches_the_page() {
    // Given
    let mounted = laid_out();
    assert_eq!(count(&mounted, "kpi"), KPI_COUNT);

    // When / Then: every item shows the page `page` names
    for (item, name) in NAV_ITEMS.iter().enumerate() {
        nav(&mounted, item);
        let shown = |class| count(&mounted, class);
        let want = |p| usize::from(page(item) == p);
        assert_eq!(shown("kpis"), want(Page::Overview), "{name}");
        assert_eq!(shown("feed"), want(Page::Messages), "{name}");
        assert_eq!(shown("empty"), want(Page::Empty), "{name}");
    }
}

#[test]
fn an_empty_page_shows_its_label_across_the_content() {
    // Given
    let mounted = laid_out();

    // When
    nav(&mounted, 2);

    // Then
    assert_eq!(texts(&mounted, "empty"), [EMPTY_LABEL]);
    let doc = mounted.document.borrow();
    let [x, y, w, _] = rect(&doc, by_class(&doc, "empty")[0]);
    assert_eq!(
        [x, y, w],
        [
            SIDEBAR_WIDTH + CONTENT_PADDING,
            TOPBAR_HEIGHT + EMPTY_TOP,
            WINDOW.0 - SIDEBAR_WIDTH - 2.0 * CONTENT_PADDING
        ]
    );
    let style = doc
        .computed_style(by_class(&doc, "empty")[0])
        .expect("styled");
    assert_eq!(
        style
            .get_by_name("text-align")
            .expect("a property")
            .to_css(),
        "center"
    );
}

/// Table: each message card's boxes against the spec's measures.
#[test]
fn every_message_card_is_where_the_spec_puts_it() {
    // Given
    let mounted = laid_out();
    nav(&mounted, MESSAGES_ITEM);
    let doc = mounted.document.borrow();
    let cards = by_class(&doc, "message");
    assert_eq!(cards.len(), FEED_ROWS);
    let mut top = TOPBAR_HEIGHT + CONTENT_PADDING;

    // Then
    for (i, card) in cards.iter().enumerate() {
        let [x, y, w, h] = rect(&doc, *card);
        let body = rect(&doc, by_class(&doc, "body")[i]);
        let avatar = rect(&doc, by_class(&doc, "message-avatar")[i]);
        let author = rect(&doc, by_class(&doc, "author")[i]);
        let time = rect(&doc, by_class(&doc, "time")[i]);
        let lines = body[3] / MESSAGE_LINE;
        let (down, across) = MESSAGE_PADDING;
        assert_eq!(
            [x, y, w],
            [
                SIDEBAR_WIDTH + CONTENT_PADDING,
                top,
                WINDOW.0 - SIDEBAR_WIDTH - 2.0 * CONTENT_PADDING
            ],
            "card {i}"
        );
        assert!(
            lines >= 1.0 && lines.fract() == 0.0,
            "card {i} body of {lines} lines"
        );
        assert_eq!(h, 2.0 * down + MESSAGE_LINE + body[3], "card {i} height");
        assert_eq!(
            avatar,
            [x + across, y + down, MESSAGE_AVATAR, MESSAGE_AVATAR],
            "avatar {i}"
        );
        assert_eq!(
            [author[0], author[1]],
            [x + MESSAGE_TEXT_X, y + down],
            "author {i}"
        );
        assert_eq!(time[0], author[0] + author[2] + 8.0, "time {i}");
        assert_eq!(
            [body[0], body[1]],
            [x + MESSAGE_TEXT_X, y + down + MESSAGE_LINE],
            "body {i}"
        );
        assert!(
            body[0] + body[2] <= x + w - across + 0.01,
            "body {i} wraps at the right padding"
        );
        top += h + MESSAGE_GAP;
    }
}

#[test]
fn the_cards_show_the_feeds_texts_and_long_bodies_wrap() {
    // Given
    let mounted = laid_out();
    nav(&mounted, MESSAGES_ITEM);

    // Then
    let shown = texts(&mounted, "body");
    let doc = mounted.document.borrow();
    for (i, body) in shown.iter().enumerate() {
        assert_eq!(*body, message(i).body, "body {i}");
        assert_eq!(text(&doc, by_class(&doc, "author")[i]), message(i).author);
        assert_eq!(text(&doc, by_class(&doc, "time")[i]), message(i).time);
    }
    let three_sentences = rect(&doc, by_class(&doc, "body")[2]);
    assert!(
        three_sentences[3] > MESSAGE_LINE,
        "message 2 is three sentences and wraps"
    );
}

#[test]
fn a_narrower_window_makes_the_cards_taller() {
    // Given
    let mounted = laid_out();
    nav(&mounted, MESSAGES_ITEM);
    let before = rect(
        &mounted.document.borrow(),
        by_class(&mounted.document.borrow(), "message")[2],
    )[3];

    // When
    let viewport = vapors_elements::Viewport {
        width: 700.0,
        height: WINDOW.1,
        scale: 1.0,
    };
    mounted
        .document
        .borrow_mut()
        .update(viewport)
        .expect("lays out");

    // Then
    let doc = mounted.document.borrow();
    assert!(rect(&doc, by_class(&doc, "message")[2])[3] > before);
}
