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
    let item = 2;
    assert_eq!(page(item), Page::Empty);

    // When
    nav(&mounted, item);

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

#[test]
fn every_avatar_has_the_background_of_its_colour() {
    // Given
    assert_eq!(AVATAR_COLORS.len(), 8, "the palette binds 8 colours");
    let mounted = with_rows(AVATAR_COLORS.len() + 1);
    nav(&mounted, MESSAGES_ITEM);

    // Then
    let doc = mounted.document.borrow();
    let avatars = by_class(&doc, "message-avatar");
    assert_eq!(avatars.len(), AVATAR_COLORS.len() + 1);
    for (i, avatar) in avatars.iter().enumerate() {
        let [_, r, g, b] = message(i).avatar.to_be_bytes();
        let style = doc.computed_style(*avatar).expect("styled");
        let got = style.get_by_name("background-color").expect("a property");
        assert_eq!(got.to_css(), format!("rgb({r} {g} {b})"), "avatar {i}");
    }
}

#[test]
fn the_feed_holds_as_many_cards_as_rows_for_any_count() {
    // 0 is an empty feed; 50 is past every list in the data and the feed
    // is defined there (the spec's `message` wraps its lists by index).
    for rows in [0, 1, FEED_ROWS + 1, 50] {
        // Given
        let mounted = with_rows(rows);

        // When
        nav(&mounted, MESSAGES_ITEM);

        // Then
        assert_eq!(count(&mounted, "message"), rows, "{rows} rows");
        let doc = mounted.document.borrow();
        for (i, author) in by_class(&doc, "author").iter().enumerate() {
            assert_eq!(text(&doc, *author), message(i).author, "{rows}: {i}");
        }
    }
}

#[test]
fn the_rows_argument_reaches_the_mounted_feed() {
    // Given: what `--rows 7` hands `run`
    let mounted = crate::app::mount(7);
    refresh(&mounted);

    // When
    nav(&mounted, MESSAGES_ITEM);

    // Then
    assert_eq!(count(&mounted, "message"), 7);
}
