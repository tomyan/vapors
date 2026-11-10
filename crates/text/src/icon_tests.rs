use super::*;

/// An icon of one stroke in a 16 by 16 view box.
fn icon(data: &str, width: f32) -> Icon {
    icon_in([0.0, 0.0, 16.0, 16.0], &[(data, width)])
}

fn icon_in(view_box: [f32; 4], strokes: &[(&str, f32)]) -> Icon {
    let strokes = strokes
        .iter()
        .map(|(data, width)| IconStroke {
            path: IconPath::parse(data).expect("parses"),
            width: *width,
        })
        .collect();
    Icon::new(view_box, strokes).expect("a valid icon")
}

fn at(coverage: &Coverage, x: u32, y: u32) -> u8 {
    coverage.data[(y * coverage.width + x) as usize]
}

/// Coverage of column `x`, rows `0..height`.
fn column(coverage: &Coverage, x: u32) -> Vec<u8> {
    (0..coverage.height).map(|y| at(coverage, x, y)).collect()
}

fn rows_inked(coverage: &Coverage, x: u32) -> Vec<u32> {
    (0..coverage.height)
        .filter(|&y| at(coverage, x, y) > 0)
        .collect()
}

#[test]
fn a_stroke_covers_its_width_about_the_path_at_the_device_size() {
    // Given: a 2-unit line across y = 8
    let icon = icon("M0 8H16", 2.0);

    // When
    let coverage = icon.rasterize([16, 16]);

    // Then: a 16 by 16 bitmap at the box's corner, rows 7 and 8 full
    assert_eq!([coverage.left, coverage.top], [0, 0]);
    assert_eq!([coverage.width, coverage.height], [16, 16]);
    let mut expected = vec![0; 16];
    expected[7] = 255;
    expected[8] = 255;
    assert_eq!(column(&coverage, 5), expected);
}

#[test]
fn view_box_units_scale_to_device_pixels_stroke_width_included() {
    // Given
    let icon = icon("M0 8H16", 2.0);

    // When: twice the view box
    let coverage = icon.rasterize([32, 32]);

    // Then: rows 14 to 17, four device pixels of stroke
    assert_eq!(rows_inked(&coverage, 5), vec![14, 15, 16, 17]);
}

#[test]
fn the_view_box_origin_maps_to_the_box_corner() {
    // Given: the same line in a view box starting at (8, 4)
    let icon = icon_in([8.0, 4.0, 16.0, 16.0], &[("M8 12H24", 2.0)]);

    // When
    let coverage = icon.rasterize([16, 16]);

    // Then
    assert_eq!(rows_inked(&coverage, 5), vec![7, 8]);
}

#[test]
fn overlapping_strokes_combine_as_source_over() {
    // Given: a 1-unit stroke centred on a pixel edge, half covering rows 7 and 8
    let one = icon("M0 8H16", 1.0).rasterize([16, 16]);
    let twice = icon_in(
        [0.0, 0.0, 16.0, 16.0],
        &[("M0 8H16", 1.0), ("M0 8H16", 1.0)],
    )
    .rasterize([16, 16]);

    // When
    let (single, double) = (f32::from(at(&one, 5, 7)), f32::from(at(&twice, 5, 7)));

    // Then: c + c(1 - c), as two paths of one colour paint in SVG
    let expected = single + single * (1.0 - single / 255.0);
    assert!((126.0..=129.0).contains(&single), "{single}");
    assert!((double - expected).abs() <= 1.0, "{double} vs {expected}");
}

#[test]
fn joins_are_miters_as_in_svg() {
    // Given: a right angle with its corner at (2, 2)
    let icon = icon("M2 14V2H14", 2.0);

    // When
    let coverage = icon.rasterize([16, 16]);

    // Then: the outside corner pixel is fully covered (a round or bevel join leaves it partial)
    assert_eq!(at(&coverage, 1, 1), 255);
}

#[test]
fn caps_are_butt_as_in_svg() {
    // Given: a line from x 4 to 12
    let icon = icon("M4 8H12", 2.0);

    // When
    let coverage = icon.rasterize([16, 16]);

    // Then: nothing beyond either end (a square cap would cover columns 3 and 12)
    assert_eq!(rows_inked(&coverage, 3), Vec::<u32>::new());
    assert_eq!(rows_inked(&coverage, 12), Vec::<u32>::new());
    assert_eq!(rows_inked(&coverage, 4), vec![7, 8]);
}

#[test]
fn an_icon_is_refused_when_its_view_box_has_no_area() {
    // Given
    let path = IconPath::parse("M0 0H1").expect("parses");
    let strokes = vec![IconStroke { path, width: 1.0 }];

    // When
    let error = Icon::new([0.0, 0.0, 0.0, 16.0], strokes).expect_err("refused");

    // Then
    assert!(error.to_string().contains("Fix: give viewBox"), "{error}");
}

#[test]
fn an_icon_is_refused_when_a_stroke_width_is_not_positive() {
    for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        // Given
        let path = IconPath::parse("M0 0H1").expect("parses");
        let strokes = vec![IconStroke { path, width }];

        // When
        let error = Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect_err("refused");

        // Then
        assert!(
            matches!(error, IconError::StrokeWidth { index: 0, .. }),
            "{error:?}"
        );
        assert!(
            error.to_string().contains("Fix: give stroke-width"),
            "{error}"
        );
    }
}

#[test]
fn a_box_snapped_to_another_ratio_scales_the_view_box_uniformly_and_centres_it() {
    // Given: a 16 by 16 view box in a 16 by 18 box
    let icon = icon("M0 8H16", 2.0);

    // When
    let coverage = icon.rasterize([16, 18]);

    // Then: scale 1 and one pixel above, so rows 8 and 9 full (a stretch gives partial rows 7 and 10)
    let mut expected = vec![0; 18];
    expected[8] = 255;
    expected[9] = 255;
    assert_eq!(column(&coverage, 5), expected);
}

#[test]
fn a_join_with_miter_ratio_3_is_mitered_under_svgs_limit_of_4() {
    // Given: legs 19.47 degrees either side of vertical, apex (8, 4); the
    // miter reaches 3 half-widths past the apex, to y = 1
    let icon = icon("M4.464 14L8 4L11.536 14", 2.0);

    // When
    let coverage = icon.rasterize([16, 16]);

    // Then: row 2 beside the apex is about half covered (a bevel, as a limit of 2 gives, leaves it empty)
    assert!(at(&coverage, 7, 2) > 100, "{}", at(&coverage, 7, 2));
    assert!(at(&coverage, 8, 2) > 100, "{}", at(&coverage, 8, 2));
}

#[test]
fn an_icon_is_refused_when_it_has_no_strokes() {
    // When
    let error = Icon::new([0.0, 0.0, 16.0, 16.0], Vec::new()).expect_err("refused");

    // Then
    assert_eq!(error, IconError::NoStrokes);
    assert!(error.to_string().contains("Fix: give the <svg>"), "{error}");
}
