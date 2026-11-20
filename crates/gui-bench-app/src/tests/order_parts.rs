//! The refusals of an order row: an order its CSS cannot show panics.
use crate::dashboard::{Order, Status, order};
use crate::order_parts::{FULL, PILLS, parts, pill};

#[test]
fn a_good_order_gives_its_classes() {
    let o = Order {
        progress: FULL,
        ..order(1)
    };
    assert_eq!(parts(&o), (5, 3, FULL));
}

#[test]
#[should_panic(expected = "avatar colour not in AVATAR_COLORS")]
fn an_avatar_colour_outside_the_table_is_refused() {
    parts(&Order {
        avatar: 0x0012_3456,
        ..order(0)
    });
}

#[test]
#[should_panic(expected = "is not a percentage")]
fn a_progress_above_100_is_refused() {
    parts(&Order {
        progress: FULL + 1,
        ..order(0)
    });
}

#[test]
#[should_panic(expected = "has no pill class")]
fn a_status_past_the_pill_classes_is_refused() {
    pill(PILLS);
}

#[test]
fn every_status_has_a_pill_class() {
    assert_eq!(Status::ALL.len(), PILLS);
    for s in Status::ALL {
        assert!(s.filter_row() < PILLS);
    }
}
