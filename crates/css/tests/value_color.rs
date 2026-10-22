//! Colour values. Translated from sumi's `runtime/css/color_test.go`; sumi
//! kept the eight ANSI keywords as palette names, vapors resolves them to
//! their CSS Color 4 sRGB values. Alpha is kept rather than dropped.

use vapors_css::value::{Color, ColorScheme, parse_color_text};
use vapors_scene::Color as Rgba;

fn rgb_near(got: Rgba, r: u8, g: u8, b: u8, label: &str) {
    let [gr, gg, gb, _] = got.to_rgba8();
    for (name, got, want) in [("R", gr, r), ("G", gg, g), ("B", gb, b)] {
        let diff = i32::from(got) - i32::from(want);
        assert!(diff.abs() <= 3, "{label}: {name} = {got}, want ~{want}");
    }
}

fn srgb(text: &str) -> Rgba {
    match parse_color_text(text) {
        Ok(Color::Rgba(c)) => c,
        Ok(other) => panic!("{text}: not an sRGB colour: {other:?}"),
        Err(e) => panic!("{text}: {e}"),
    }
}

#[test]
fn ansi_keywords_are_css_named_colours() {
    rgb_near(srgb("cyan"), 0, 255, 255, "cyan");
    rgb_near(srgb("green"), 0, 128, 0, "green");
}

#[test]
fn parse_named_color() {
    rgb_near(srgb("rebeccapurple"), 0x66, 0x33, 0x99, "rebeccapurple");
    rgb_near(srgb("tomato"), 0xff, 0x63, 0x47, "tomato");
    rgb_near(srgb("Tomato"), 0xff, 0x63, 0x47, "case-insensitive");
}

#[test]
fn parse_hex_forms() {
    rgb_near(srgb("#ff5500"), 255, 85, 0, "#rrggbb");
    rgb_near(srgb("#f50"), 255, 85, 0, "#rgb");
    rgb_near(srgb("#ff550080"), 255, 85, 0, "#rrggbbaa");
    rgb_near(srgb("#f508"), 255, 85, 0, "#rgba");
}

#[test]
fn parse_rgb_functions() {
    rgb_near(srgb("rgb(255, 85, 0)"), 255, 85, 0, "legacy rgb");
    rgb_near(srgb("rgb(255 85 0)"), 255, 85, 0, "modern rgb");
    rgb_near(srgb("rgba(255, 85, 0, 0.5)"), 255, 85, 0, "rgba");
    rgb_near(srgb("rgb(100% 0% 50%)"), 255, 0, 128, "percentage rgb");
    rgb_near(srgb("rgb(255 85 0 / 0.5)"), 255, 85, 0, "slash alpha");
}

#[test]
fn parse_hsl_functions() {
    rgb_near(srgb("hsl(0, 100%, 50%)"), 255, 0, 0, "red hsl");
    rgb_near(srgb("hsl(120 100% 25%)"), 0, 128, 0, "green hsl");
    rgb_near(srgb("hsl(240deg 100% 50%)"), 0, 0, 255, "blue hsl deg");
    rgb_near(srgb("hsl(0.5turn 100% 50%)"), 0, 255, 255, "turn hue");
}

#[test]
fn parse_hwb() {
    rgb_near(srgb("hwb(0 0% 0%)"), 255, 0, 0, "pure red hwb");
    rgb_near(srgb("hwb(0 100% 0%)"), 255, 255, 255, "all white");
    rgb_near(srgb("hwb(0 0% 100%)"), 0, 0, 0, "all black");
}

#[test]
fn parse_lab() {
    rgb_near(srgb("lab(54.29 80.81 69.89)"), 255, 0, 0, "lab red");
    rgb_near(srgb("lch(54.29 106.84 40.86)"), 255, 0, 0, "lch red");
}

#[test]
fn parse_oklab() {
    rgb_near(srgb("oklab(0.628 0.2249 0.1258)"), 255, 0, 0, "oklab red");
    rgb_near(srgb("oklch(0.628 0.2577 29.23)"), 255, 0, 0, "oklch red");
    rgb_near(srgb("oklch(70% 0.1 200)"), 64, 177, 183, "oklch teal-ish");
}

#[test]
fn parse_transparent() {
    // When
    let c = srgb("transparent");

    // Then
    assert_eq!(c.to_rgba8(), [0, 0, 0, 0]);
}

#[test]
fn parse_invalid_colors_refused() {
    for v in [
        "",
        "notacolor",
        "#12",
        "#12345",
        "rgb(a b c)",
        "blah(1 2 3)",
        "rgb(1 2)",
    ] {
        assert!(parse_color_text(v).is_err(), "{v:?} should be refused");
    }
}

#[test]
fn parse_light_dark() {
    // When
    let c = parse_color_text("light-dark(#fff, rgb(0 0 0))").expect("parses");

    // Then
    let Color::LightDark(light, dark) = &c else {
        panic!("light-dark should keep both arms: {c:?}");
    };
    rgb_near(
        light.resolve(ColorScheme::Light, Rgba::from_rgba8(0, 0, 0, 255)),
        255,
        255,
        255,
        "light arm",
    );
    rgb_near(
        dark.resolve(ColorScheme::Dark, Rgba::from_rgba8(0, 0, 0, 255)),
        0,
        0,
        0,
        "dark arm",
    );
    rgb_near(
        c.resolve(ColorScheme::Dark, Rgba::from_rgba8(0, 0, 0, 255)),
        0,
        0,
        0,
        "picks dark",
    );
}

#[test]
fn parse_light_dark_invalid_arm_refused() {
    assert!(parse_color_text("light-dark(bogus, #000)").is_err());
}

#[test]
fn parse_color_alpha_kept() {
    for (text, want) in [
        ("#ff000080", 128),
        ("rgb(255 0 0 / 0.5)", 128),
        ("rgb(255 0 0 / 50%)", 128),
        ("#ff0000ff", 255),
        ("rgb(255 0 0)", 255),
        ("#ff000000", 0),
    ] {
        assert_eq!(srgb(text).to_rgba8()[3], want, "{text}");
    }
}

#[test]
fn current_color_resolves_to_the_current_colour() {
    // When
    let c = parse_color_text("currentColor").expect("parses");

    // Then
    assert_eq!(c, Color::CurrentColor);
    let current = Rgba::from_rgba8(1, 2, 3, 255);
    assert_eq!(c.resolve(ColorScheme::Light, current), current);
}

#[test]
fn none_components_read_as_zero() {
    rgb_near(srgb("rgb(none 255 none)"), 0, 255, 0, "none");
}

#[test]
fn oklab_round_trips_srgb() {
    // Given
    let c = Rgba::from_rgba8(200, 30, 90, 128);

    // When
    let back = Color::from_oklab(Color::to_oklab(c));

    // Then
    assert_eq!(back.to_rgba8(), c.to_rgba8());
}

#[test]
fn oklab_lightness_of_white_is_one() {
    let [l, a, b, _] = Color::to_oklab(Rgba::from_rgba8(255, 255, 255, 255));
    assert!(
        (l - 1.0).abs() < 0.001 && a.abs() < 0.001 && b.abs() < 0.001,
        "{l} {a} {b}"
    );
}
