//! GPU tier: an `svg` in a document is drawn by `ScenePainter` through
//! `TextPipeline::draw_icons` (docs/icons.md, Paint): the same pixels as
//! a direct call with the same icon at the snapped content box, at 1x
//! 1.5x and 2x; cut by a clipping ancestor; nothing when hidden; and a
//! translucent colour compounds where two paths overlap.

use vapors_app::ScenePainter;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::{Color, Size};
use vapors_shapes::IconPath;
use vapors_text::{AtlasConfig, FontSystem, GlyphAtlas, Icon, IconDraw, IconStroke, TextPipeline};

const SIZE: u32 = 48;
const WHITE: [u8; 4] = [255, 255, 255, 255];
const UNDO: [&str; 2] = ["M5 3L2 6l3 3", "M2 6h7a4 4 0 0 1 0 8H6"];
const CROSS: [&str; 2] = ["M0 8H16", "M8 0V16"];

/// A body styled `css` holding a `div.frame` holding an `svg.icon` of
/// 16 x 16 whose paths are `paths`, `stroke_width` wide.
fn page(css: &str, paths: [&str; 2], stroke_width: &str) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let frame = child(&mut doc, body, "div", &[("class", "frame")]);
    let size = [("viewBox", "0 0 16 16"), ("width", "16"), ("height", "16")];
    let svg = child(&mut doc, frame, "svg", &size);
    doc.set_attribute(svg, "class", "icon");
    doc.set_attribute(svg, "aria-hidden", "true");
    for d in paths {
        let stroke = [("fill", "none"), ("stroke", "currentColor")];
        let path = child(&mut doc, svg, "path", &stroke);
        doc.set_attribute(path, "d", d);
        doc.set_attribute(path, "stroke-width", stroke_width);
    }
    let css = format!("body {{ margin: 0; }} .icon {{ display: block; }} {css}");
    doc.add_stylesheet("app.css", &css).expect("parses");
    doc
}

fn child(doc: &mut Document, parent: NodeId, tag: &str, attributes: &[(&str, &str)]) -> NodeId {
    let node = doc.create_element(tag);
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
    doc.append(parent, node);
    node
}

/// `doc` drawn by a `ScenePainter` at `scale` on a white target.
fn drawn(doc: &mut Document, scale: f32) -> Vec<u8> {
    #[allow(clippy::cast_precision_loss)] // a small size
    let side = SIZE as f32 / scale;
    let viewport = Viewport {
        width: side,
        height: side,
        scale,
    };
    let update = doc.update(viewport).expect("updates");
    on_white(|gpu, target| {
        let mut painter = ScenePainter::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm)
            .expect("Rgba8Unorm is drawn");
        painter.draw(target, update).expect("paints");
    })
}

/// `icon` drawn directly by `TextPipeline::draw_icons` on a white target.
fn drawn_directly(icon: &Icon, origin: [i32; 2], size: [u32; 2], colour: Color) -> Vec<u8> {
    on_white(|gpu, target| {
        let mut atlas = GlyphAtlas::new(AtlasConfig::default());
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut pipeline = TextPipeline::new(&gpu.device, format, &atlas);
        let draw = IconDraw {
            icon,
            origin,
            size,
            colour,
        };
        let drawn = pipeline.draw_icons(target, &mut atlas, &[draw]);
        drawn.expect("draws");
    })
}

/// The pixels of a white `SIZE` square after `draw` draws on it.
fn on_white(draw: impl FnOnce(&vapors_gpu_harness::Gpu, &Target<'_>)) -> Vec<u8> {
    let gpu = gpu();
    let texture = clear(
        &gpu.device,
        &gpu.queue,
        SIZE,
        SIZE,
        Color::from_rgba8(255, 255, 255, 255),
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport: Size {
            width: SIZE,
            height: SIZE,
        },
        scissor: None,
    };
    draw(&gpu, &target);
    encoder.submit(&gpu.queue).expect("no draw refused");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

fn icon(paths: [&str; 2], width: f32) -> Icon {
    let stroke = |d| IconStroke {
        path: IconPath::parse(d).expect("parses"),
        width,
    };
    Icon::new([0.0, 0.0, 16.0, 16.0], paths.map(stroke).to_vec()).expect("a valid icon")
}

fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
}

/// The pixels that are not white, as (x, y).
fn inked(rgba: &[u8]) -> Vec<(u32, u32)> {
    let all = (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| (x, y)));
    all.filter(|&(x, y)| pixel(rgba, x, y) != WHITE).collect()
}

#[test]
fn gpu_an_svg_draws_the_pixels_draw_icons_draws_at_its_snapped_content_box() {
    for (scale, origin, size) in [
        (1.0, [3, 3], [16, 16]),
        (1.5, [5, 5], [24, 24]),
        (2.0, [7, 7], [32, 32]),
    ] {
        // Given: content at 3.25 px, snapped to 3 at 1x, 5 (4.875) and 29 (28.875) at 1.5x, 7 (6.5) at 2x
        let css = ".frame { color: rgb(200, 30, 60); } .icon { margin: 3.25px; }";
        let mut doc = page(css, UNDO, "1.5");

        // When
        let rgba = drawn(&mut doc, scale);

        // Then: the same pixels, in the colour
        let colour = Color::from_rgba8(200, 30, 60, 255);
        let expected = drawn_directly(&icon(UNDO, 1.5), origin, size, colour);
        assert!(!inked(&expected).is_empty(), "the icon draws at {scale}x");
        assert!(rgba == expected, "differs from draw_icons at {scale}x");
        let solid = inked(&rgba)
            .iter()
            .any(|&(x, y)| pixel(&rgba, x, y) == [200, 30, 60, 255]);
        assert!(solid, "some pixel is wholly the colour at {scale}x");
    }
}

#[test]
fn gpu_a_clipping_ancestor_cuts_the_icon() {
    // Given: an 8 x 8 frame that clips the 16 x 16 undo icon
    let css = ".frame { width: 8px; height: 8px; overflow: hidden; }";
    let mut doc = page(css, UNDO, "1.5");

    // When
    let rgba = drawn(&mut doc, 1.0);

    // Then: ink inside the frame, none outside it
    let found = inked(&rgba);
    assert!(!found.is_empty(), "the arrow inside the frame draws");
    let outside: Vec<_> = found.iter().filter(|&&(x, y)| x >= 8 || y >= 8).collect();
    assert_eq!(outside, Vec::<&(u32, u32)>::new());
}

#[test]
fn gpu_a_hidden_svg_draws_nothing() {
    // Given
    let mut doc = page(".icon { visibility: hidden; }", UNDO, "1.5");

    // When
    let rgba = drawn(&mut doc, 1.0);

    // Then
    assert_eq!(inked(&rgba), []);
}

#[test]
fn gpu_a_translucent_colour_compounds_where_two_paths_cross() {
    // Given: a 4-unit horizontal and vertical stroke in black at half alpha
    let mut doc = page(".icon { color: rgba(0, 0, 0, 0.5); }", CROSS, "4");

    // When
    let rgba = drawn(&mut doc, 1.0);

    // Then: one path leaves half the white, the crossing a quarter
    // (a + a(1 - a) = 0.75 covered), as Chromium paints each path
    let near = |found: [u8; 4], want: u8| found[..3].iter().all(|&c| c.abs_diff(want) <= 1);
    assert!(
        near(pixel(&rgba, 2, 8), 128),
        "one path: {:?}",
        pixel(&rgba, 2, 8)
    );
    assert!(
        near(pixel(&rgba, 8, 8), 64),
        "crossing: {:?}",
        pixel(&rgba, 8, 8)
    );
}
