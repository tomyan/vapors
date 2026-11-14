use super::*;

const SIZE: [f32; 2] = [100.0, 40.0];
const WINDOW: [f32; 2] = [800.0, 600.0];
const POINTER: Anchor = Anchor::Pointer(Point { x: 1.0, y: 1.0 });
const BOX: BoxRect = BoxRect {
    x: 0.0,
    y: 0.0,
    width: 5.0,
    height: 5.0,
};

fn refusal(anchor: Anchor, size: [f32; 2], window: [f32; 2]) -> PlacementError {
    check(anchor, size, window).unwrap_err()
}

fn keyboard(edit: impl FnOnce(&mut BoxRect)) -> Anchor {
    let mut b = BOX;
    edit(&mut b);
    Anchor::Keyboard(b)
}

fn pointer(x: f32, y: f32) -> Anchor {
    Anchor::Pointer(Point { x, y })
}

#[test]
#[allow(clippy::too_many_lines)] // one table, a case per field
fn every_bad_field_is_refused_naming_the_field() {
    // Given one bad value in each field
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    let cases = [
        (pointer(nan, 1.0), SIZE, WINDOW, PlacementField::HotspotX),
        (pointer(1.0, inf), SIZE, WINDOW, PlacementField::HotspotY),
        (
            keyboard(|b| b.x = nan),
            SIZE,
            WINDOW,
            PlacementField::AnchorX,
        ),
        (
            keyboard(|b| b.y = -inf),
            SIZE,
            WINDOW,
            PlacementField::AnchorY,
        ),
        (
            keyboard(|b| b.width = nan),
            SIZE,
            WINDOW,
            PlacementField::AnchorWidth,
        ),
        (
            keyboard(|b| b.width = -1.0),
            SIZE,
            WINDOW,
            PlacementField::AnchorWidth,
        ),
        (
            keyboard(|b| b.height = inf),
            SIZE,
            WINDOW,
            PlacementField::AnchorHeight,
        ),
        (
            keyboard(|b| b.height = -1.0),
            SIZE,
            WINDOW,
            PlacementField::AnchorHeight,
        ),
        (POINTER, [nan, 5.0], WINDOW, PlacementField::SizeWidth),
        (POINTER, [-1.0, 5.0], WINDOW, PlacementField::SizeWidth),
        (POINTER, [5.0, inf], WINDOW, PlacementField::SizeHeight),
        (POINTER, [5.0, -1.0], WINDOW, PlacementField::SizeHeight),
        (POINTER, SIZE, [nan, 600.0], PlacementField::WindowWidth),
        (POINTER, SIZE, [inf, 600.0], PlacementField::WindowWidth),
        (POINTER, SIZE, [-1.0, 600.0], PlacementField::WindowWidth),
        (POINTER, SIZE, [800.0, nan], PlacementField::WindowHeight),
        (POINTER, SIZE, [800.0, inf], PlacementField::WindowHeight),
        (POINTER, SIZE, [800.0, -1.0], PlacementField::WindowHeight),
    ];
    for (anchor, size, window, field) in cases {
        // When checked, then the field is named
        assert_eq!(refusal(anchor, size, window).field, field, "{field:?}");
    }
}

#[test]
fn a_negative_anchor_coordinate_is_accepted() {
    // Given an anchor partly off-screen
    // When checked, then it is not refused
    assert!(check(keyboard(|b| b.x = -30.0), SIZE, WINDOW).is_ok());
    assert!(check(pointer(-30.0, -30.0), SIZE, WINDOW).is_ok());
}

#[test]
fn refusal_message_names_the_field_and_value_and_the_fix() {
    let nan = refusal(keyboard(|b| b.width = f32::NAN), SIZE, WINDOW).to_string();
    let negative = refusal(POINTER, [5.0, -1.0], WINDOW).to_string();
    assert_eq!(
        nan,
        "place_tooltip: `anchor.width` is NaN; pass a finite length of 0 or more in CSS pixels"
    );
    assert_eq!(
        negative,
        "place_tooltip: `size.height` is -1; pass a finite length of 0 or more in CSS pixels"
    );
}
