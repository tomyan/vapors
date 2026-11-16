//! Where a followed link opens, by the modifiers on its click or its
//! Enter, as in Chromium: here unmodified, a new tab with Control or
//! Meta, a new window with Shift, a download with Alt.

use crate::support;

use support::link_page::{click_with, followed, key, key_with, page};
use vapors_elements::{Modifiers, Opened};

const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    meta: false,
};

/// Each modifier set and where it opens a link.
const CASES: [(Modifiers, Opened); 6] = [
    (NONE, Opened::Here),
    (
        Modifiers {
            control: true,
            ..NONE
        },
        Opened::NewTab,
    ),
    (Modifiers { meta: true, ..NONE }, Opened::NewTab),
    (
        Modifiers {
            shift: true,
            ..NONE
        },
        Opened::NewWindow,
    ),
    (Modifiers { alt: true, ..NONE }, Opened::Download),
    (
        Modifiers {
            control: true,
            shift: true,
            ..NONE
        },
        Opened::NewTab,
    ),
];

#[test]
fn a_click_opens_the_link_where_its_modifiers_say() {
    for (modifiers, opened) in CASES {
        // Given
        let page = page();

        // When
        click_with(&page, [5.0, 5.0], modifiers);

        // Then
        let link = followed(&page).expect("the link is followed");
        assert_eq!(
            (link.href.as_str(), link.opened),
            ("?demo=counter", opened),
            "{modifiers:?}"
        );
    }
}

#[test]
fn enter_opens_a_focused_link_where_its_modifiers_say() {
    for (modifiers, opened) in CASES {
        // Given: the link focused by Tab
        let page = page();
        key(&page, "Tab");

        // When
        key_with(&page, "Enter", modifiers);

        // Then
        let link = followed(&page).expect("the link is followed");
        assert_eq!(link.opened, opened, "{modifiers:?}");
    }
}
