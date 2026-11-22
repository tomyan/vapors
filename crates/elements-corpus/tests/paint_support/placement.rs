//! Placement only: in each element's region, the first and last rows and
//! columns holding dark ink (every channel under `DARK`), the same in
//! vapors' paint and Chromium's, for a fixture whose pixels are not
//! compared at a scale (`only_1x` in `gpu_paint.rs`): its glyphs and boxes
//! still sit where Chromium puts them, a device pixel off in any
//! direction fails. Anti-aliasing that differs (#388) moves no core
//! pixel of a glyph across `DARK`.

use vapors_elements_corpus::paint_check::Region;
use vapors_text::comparison::Rgba;

/// A pixel with every channel under this is ink.
const DARK: u8 = 120;

/// The ink's extent in a region: top, bottom, left, right; `None` for
/// a region with none.
pub type Extent = Option<[u32; 4]>;

/// Each region's label and its ink's extent in `image`.
pub fn inked(image: &Rgba, regions: &[Region]) -> Vec<(String, Extent)> {
    regions
        .iter()
        .map(|region| (region.label.clone(), extent(image, region.rect)))
        .collect()
}

fn extent(image: &Rgba, [left, top, right, bottom]: [u32; 4]) -> Extent {
    let dark = |x: u32, y: u32| {
        let at = ((y * image.width + x) * 4) as usize;
        image.data[at..at + 3].iter().all(|&channel| channel < DARK)
    };
    let (right, bottom) = (right.min(image.width), bottom.min(image.height));
    let points: Vec<(u32, u32)> = (top..bottom)
        .flat_map(|y| (left..right).map(move |x| (x, y)))
        .filter(|&(x, y)| dark(x, y))
        .collect();
    let ys = points.iter().map(|p| p.1);
    let xs = points.iter().map(|p| p.0);
    Some([ys.clone().min()?, ys.max()?, xs.clone().min()?, xs.max()?])
}

/// Each region whose ink spans other rows or columns in `ours` than in
/// `theirs`, with both extents.
pub fn off(ours: &Rgba, theirs: &Rgba, regions: &[Region]) -> Vec<String> {
    inked(ours, regions)
        .iter()
        .zip(&inked(theirs, regions))
        .filter(|(a, b)| a != b)
        .map(|((label, a), (_, b))| format!("{label}: ours {a:?}, Chromium's {b:?}"))
        .collect()
}

/// `image` moved down one row, its top row white: every glyph a device
/// pixel lower.
pub fn shifted_down(image: &Rgba) -> Rgba {
    let row = (image.width * 4) as usize;
    let mut data = vec![255; row];
    data.extend_from_slice(&image.data[..image.data.len() - row]);
    Rgba { data, ..*image }
}

/// `image` moved right one column, its left column white: every glyph a
/// device pixel further right.
pub fn shifted_right(image: &Rgba) -> Rgba {
    let row = (image.width * 4) as usize;
    let data = image
        .data
        .chunks(row)
        .flat_map(|line| {
            [255, 255, 255, 255]
                .into_iter()
                .chain(line[..row - 4].iter().copied())
        })
        .collect();
    Rgba { data, ..*image }
}
