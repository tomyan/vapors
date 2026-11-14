//! The icon route beside the shape methods: vapors-text's own icon
//! rasteriser (zeno, with its miter fix-ups), which draws an icon
//! axis-aligned at whole device pixels, so only the 1x and 2x
//! placements apply to it.

use vapors_shapes::IconPath;
use vapors_text::{Icon as IconMask, IconStroke};

use crate::analytic::size;
use crate::cases::{AT, CELL, Icon, PAGE, Placement};

/// `icons` drawn by the icon route in `ink`; `None` for a placement it
/// cannot draw (rotated, or at a scale that is not whole).
pub fn drawn(icons: &[&Icon], placement: &Placement, ink: [u8; 4]) -> Option<Vec<u8>> {
    if placement.css != "none" {
        return None;
    }
    let s = placement.scale;
    let viewport = size(icons.len(), placement);
    let mut page: Vec<u8> = (0..viewport.width * viewport.height)
        .flat_map(|_| PAGE)
        .collect();
    for (at, icon) in (0u32..).zip(icons) {
        let path = IconPath::parse(icon.path).expect("touche's icons parse");
        let mask = IconMask::new(
            [0.0, 0.0, 16.0, 16.0],
            vec![IconStroke {
                path,
                width: icon.stroke,
            }],
        )
        .expect("an icon")
        .rasterize([16 * s, 16 * s]);
        let origin = [(at * CELL) as f32 + AT, AT].map(|v| (v as u32) * s);
        for y in 0..mask.height {
            for x in 0..mask.width {
                let coverage = mask.data[(y * mask.width + x) as usize];
                let px = origin[0] + x;
                let py = origin[1] + y;
                let i = ((py * viewport.width + px) * 4) as usize;
                over(&mut page[i..i + 4], ink, coverage);
            }
        }
    }
    Some(page)
}

/// `ink` at `coverage` over the pixel, in its stored (sRGB-encoded)
/// values, as Chromium and the pipelines blend.
fn over(pixel: &mut [u8], ink: [u8; 4], coverage: u8) {
    let a = f32::from(ink[3]) / 255.0 * f32::from(coverage) / 255.0;
    for k in 0..3 {
        let mixed = f32::from(ink[k]) * a + f32::from(pixel[k]) * (1.0 - a);
        pixel[k] = mixed.round() as u8;
    }
}
