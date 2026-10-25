use super::*;

fn image(rgba: &[u8], width: u32) -> Image<'_> {
    Image {
        width,
        height: u32::try_from(rgba.len()).unwrap() / 4 / width,
        scale: 1.0,
        rgba,
    }
}

#[test]
fn both_shapes_parse_and_comments_are_skipped() {
    // Given
    let text = "# heading\npixel 1 2 #0a0b0c   where\n\nink 0 0 4 5 #ffffff box\n";

    // When
    let checks = parse("demo", text).expect("parses").remove(0).checks;

    // Then
    assert_eq!(
        checks,
        [
            Check::Pixel {
                x: 1,
                y: 2,
                rgb: [10, 11, 12]
            },
            Check::Ink {
                from: [0, 0],
                to: [4, 5],
                rgb: [255, 255, 255]
            },
        ]
    );
}

#[test]
fn a_malformed_line_is_refused_naming_its_line() {
    // Given
    let text = "pixel 1 2 #0a0b0c\npixel one 2 #000000\n";

    // When
    let error = parse("hello", text).expect_err("refused");

    // Then
    assert!(error.starts_with("hello.pixels:2: `one`"), "{error}");
}

#[test]
fn a_pixel_within_one_per_channel_passes_and_further_fails() {
    // Given: two pixels, 1 and 2 away from #101010
    let rgba = [17, 16, 15, 255, 18, 16, 16, 255];
    let checks = [
        Check::Pixel {
            x: 0,
            y: 0,
            rgb: [16, 16, 16],
        },
        Check::Pixel {
            x: 1,
            y: 0,
            rgb: [16, 16, 16],
        },
    ];

    // When
    let found = failures(&checks, &image(&rgba, 2));

    // Then
    assert_eq!(found, ["pixel 1,0: wanted #101010, found #121010"]);
}

#[test]
fn ink_fails_only_when_every_pixel_in_the_box_is_the_background() {
    // Given: a 2x1 image, the second pixel drawn on
    let rgba = [0, 0, 0, 255, 200, 200, 200, 255];
    let blank = Check::Ink {
        from: [0, 0],
        to: [1, 1],
        rgb: [0, 0, 0],
    };
    let drawn = Check::Ink {
        from: [0, 0],
        to: [2, 1],
        rgb: [0, 0, 0],
    };

    // When
    let found = failures(&[blank, drawn], &image(&rgba, 2));

    // Then
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("nothing was drawn"), "{found:?}");
}

#[test]
fn a_pixel_outside_the_image_fails() {
    // Given
    let rgba = [0, 0, 0, 255];

    // When
    let found = failures(
        &[Check::Pixel {
            x: 5,
            y: 0,
            rgb: [0, 0, 0],
        }],
        &image(&rgba, 1),
    );

    // Then
    assert!(found[0].contains("outside the image"), "{found:?}");
}

#[test]
fn clicks_and_wheels_are_kept_in_file_order_apart_from_the_checks() {
    // Given
    let text = "click 3 4\npixel 1 2 #000000\nwheel 5 6 0 -120\nclick 7 8\n";

    // When
    let spec = parse("demo", text).expect("parses").remove(0);

    // Then
    assert_eq!(
        spec.inputs,
        [
            Input::Click([3, 4]),
            Input::Wheel {
                at: [5, 6],
                delta: [0, -120]
            },
            Input::Click([7, 8])
        ]
    );
    assert_eq!(spec.checks.len(), 1);
}

#[test]
fn a_wheel_line_without_its_deltas_is_refused_naming_the_line() {
    // When
    let error = parse("demo", "wheel 5 6 0\n").expect_err("refused");

    // Then
    assert!(
        error.contains("demo.pixels:1") && error.contains("wheel X Y DX DY"),
        "{error}"
    );
}

#[test]
fn inputs_print_as_their_lines_for_the_page_script() {
    // Given
    let inputs = [
        Input::Click([3, 4]),
        Input::Wheel {
            at: [5, 6],
            delta: [-2, 120],
        },
    ];

    // When
    let lines: Vec<String> = inputs.iter().map(Input::line).collect();

    // Then
    assert_eq!(lines, ["click 3 4", "wheel 5 6 -2 120"]);
}

#[test]
fn an_at_line_starts_a_section_for_another_viewport() {
    // Given
    let text = "pixel 1 1 #000000\nat 412x800\nclick 2 2\npixel 3 3 #ffffff\n";

    // When
    let specs = parse("demo", text).expect("parses");

    // Then
    assert_eq!(specs.len(), 2);
    assert_eq!(
        (specs[0].viewport, specs[0].checks.len()),
        (DEFAULT_VIEWPORT, 1)
    );
    assert_eq!(
        (
            specs[1].viewport,
            specs[1].inputs.clone(),
            specs[1].checks.len()
        ),
        ([412, 800], vec![Input::Click([2, 2])], 1)
    );
    assert!(section(&specs, [412, 800]).is_ok());
    assert!(
        section(&specs, [100, 100])
            .expect_err("none")
            .contains("640x400, 412x800")
    );
}

#[test]
fn two_sections_for_one_viewport_are_refused() {
    // Given
    let text = "at 412x800\nat 412x800\n";

    // When
    let error = parse("demo", text).expect_err("refused");

    // Then
    assert!(error.contains("a second section for 412x800"), "{error}");
}
