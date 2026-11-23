use super::Window;

/// 20 px rows in a 60 px window, one row of overscan.
fn small() -> Window {
    Window::new(20.0, 60.0, 1)
}

#[test]
fn at_the_top_the_rows_in_the_window_and_the_overscan_below_exist() {
    // When
    let rows = small().rows(0.0, 20);

    // Then: rows 0..3 fill the window, row 3 is the overscan
    assert_eq!(rows, 0..4);
}

#[test]
fn a_row_partly_in_view_at_either_end_exists_with_overscan_beyond_it() {
    // When: 105..165 shows part of row 5, rows 6 and 7, and part of row 8
    let rows = small().rows(105.0, 20);

    // Then
    assert_eq!(rows, 4..10);
}

#[test]
fn at_the_end_the_rows_stop_at_the_count() {
    // When: the 340 px range's end shows rows 17..20
    let rows = small().rows(340.0, 20);

    // Then
    assert_eq!(rows, 16..20);
}

#[test]
fn an_offset_past_the_rows_gives_none() {
    // When: the keys shrank under an offset the next clamp has not reached
    let rows = small().rows(340.0, 5);

    // Then
    assert!(rows.is_empty());
}

#[test]
fn no_rows_give_no_range() {
    assert_eq!(small().rows(0.0, 0), 0..0);
}

#[test]
fn the_range_is_as_long_at_a_million_rows_as_at_twenty() {
    // Given
    let window = Window::new(52.0, 756.0, 4);

    // When
    let deep = window.rows(26_000_000.0, 1_000_000);

    // Then: 15 rows touch a 756 px window at a row edge, 4 on each side
    assert_eq!(deep.len(), 15 + 8);
}

#[test]
fn heights_are_rows_times_the_row_height() {
    assert_eq!(small().px(7), 140.0);
}

#[test]
fn a_million_rows_of_52_px_is_an_exact_height() {
    assert_eq!(Window::new(52.0, 756.0, 4).px(1_000_000), 52_000_000.0);
}

#[test]
fn a_decimal_row_height_is_accepted_up_to_7872_rows_of_33_3_px() {
    // When: 33.3 px has no exact f32, and neither do most of its products
    let window = Window::new(33.3, 60.0, 1);
    let heights = [window.px(3), window.px(7_872)];

    // Then: each within 1/128 px of the exact height
    let exact = [3.0, 7_872.0].map(|rows| rows * f64::from(33.3_f32));
    for (height, exact) in heights.iter().zip(exact) {
        assert!(
            (f64::from(*height) - exact).abs() <= 1.0 / 128.0,
            "{height}"
        );
    }
}

#[test]
#[should_panic(expected = "7873 rows of 33.3 px")]
fn a_decimal_row_height_is_refused_from_7873_rows_of_33_3_px() {
    // Where an f32 steps by 1/32 px: half that is past the 1/128 bound
    let _ = Window::new(33.3, 60.0, 1).px(7_873);
}

#[test]
#[should_panic(expected = "7874 rows of 33.3 px")]
fn a_count_past_the_first_refused_is_refused_too() {
    // 7,874 rows of 33.3 px happen to round by under 1/128 px, but 7,873
    // do not: a content height of 7,874 rows would mount, and a scroll
    // to row 7,873 would then panic
    let _ = Window::new(33.3, 60.0, 1).px(7_874);
}

#[test]
fn every_spacer_below_a_held_content_height_holds_too() {
    // Given: the most rows of 33.3 px a list holds
    let window = Window::new(33.3, 60.0, 1);
    let _ = window.px(7_872);

    // When: every spacer height a scroll can ask for, row by row
    let spacers = (0..=7_872).map(|rows| window.px(rows));

    // Then: none refused
    assert_eq!(spacers.count(), 7_873);
}

#[test]
#[should_panic(expected = "more than 1/128")]
fn a_height_an_f32_cannot_hold_is_refused() {
    let _ = Window::new(0.1, 60.0, 1).px(16_777_217);
}

#[test]
#[should_panic(expected = "row_height is 0")]
fn a_zero_row_height_is_refused() {
    let _ = Window::new(0.0, 60.0, 1);
}

#[test]
#[should_panic(expected = "row_height is NaN")]
fn a_nan_row_height_is_refused() {
    let _ = Window::new(f32::NAN, 60.0, 1);
}

#[test]
#[should_panic(expected = "height is -1")]
fn a_negative_window_height_is_refused() {
    let _ = Window::new(20.0, -1.0, 1);
}

#[test]
#[should_panic(expected = "not a clamped offset")]
fn a_negative_offset_is_refused() {
    let _ = small().rows(-1.0, 20);
}

#[test]
#[should_panic(expected = "2 rows of 340282350000000000000000000000000000000 px")]
fn a_height_past_the_largest_f32_is_refused() {
    // Two rows of the largest f32 overflow to infinity
    let _ = Window::new(f32::MAX, 60.0, 1).px(2);
}

#[cfg(target_pointer_width = "64")] // a count past 2^32
#[test]
#[should_panic(expected = "past 2^53")]
fn a_count_past_2_to_the_53_is_refused() {
    let _ = Window::new(1.0, 60.0, 1).px((1 << 53) + 1);
}

#[cfg(target_pointer_width = "64")] // a count past 2^32
#[test]
#[should_panic(expected = "6004799503160661 rows of 3 px")]
fn a_count_whose_f64_product_rounds_to_an_f32_is_not_taken_as_exact() {
    // (2^54 - 1) / 3 rows of 3 px make 2^54 - 1 px, which an f64 rounds to
    // 2^54, an f32; the true height is none, and 2^24 + 1 rows of 3 px
    // below it are refused too
    let _ = Window::new(3.0, 60.0, 1).px(((1_usize << 54) - 1) / 3);
}

#[test]
fn a_window_given_a_taller_height_holds_the_rows_of_that_height() {
    // When: 100 px shows rows 0..5
    let rows = small().with_height(100.0).rows(0.0, 20);

    // Then
    assert_eq!(rows, 0..6);
}

#[test]
fn a_window_given_a_shorter_height_holds_the_rows_of_that_height() {
    // When: 30 px down, 20 px shows part of rows 1 and 2
    let rows = small().with_height(20.0).rows(30.0, 20);

    // Then
    assert_eq!(rows, 0..4);
}

#[test]
#[should_panic(expected = "height is NaN")]
fn a_laid_out_height_that_is_not_a_number_is_refused() {
    let _ = small().with_height(f32::NAN);
}
