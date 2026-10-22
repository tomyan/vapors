//! Conversions into sRGB (0..=255 channels) from the colour spaces CSS
//! Color Level 4 defines: HSL, HWB, CIELAB (D50) and Oklab.

// The single-letter names are the specification's own channel names (h,
// s, l, a, b, x, y, z); longer names would not match the formulas.
#![allow(clippy::many_single_char_names)]

/// HSL with `s` and `l` in 0..=1.
pub(super) fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let h = h.rem_euclid(360.0) / 360.0;
    if s == 0.0 {
        return (l * 255.0, l * 255.0, l * 255.0);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    (
        hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0,
        hue_to_rgb(p, q, h) * 255.0,
        hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0,
    )
}

fn hue_to_rgb(p: f64, q: f64, t: f64) -> f64 {
    let t = t.rem_euclid(1.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// HWB with whiteness and blackness in 0..=1.
pub(super) fn hwb_to_rgb(h: f64, w: f64, b: f64) -> (f64, f64, f64) {
    if w + b >= 1.0 {
        let gray = w / (w + b) * 255.0;
        return (gray, gray, gray);
    }
    let (r, g, bl) = hsl_to_rgb(h, 1.0, 0.5);
    let scale = 1.0 - w - b;
    let mix = |c: f64| (c / 255.0 * scale + w) * 255.0;
    (mix(r), mix(g), mix(bl))
}

/// Chroma and hue (degrees) to the rectangular a, b components.
pub(super) fn polar_to_ab(c: f64, h: f64) -> (f64, f64) {
    let rad = h.to_radians();
    (c * rad.cos(), c * rad.sin())
}

/// CIELAB, D50 white as CSS specifies, to sRGB.
pub(super) fn lab_to_rgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let inv = |t: f64| {
        let t3 = t * t * t;
        if t3 > 0.008_856 {
            t3
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };
    let (x, y, z) = (inv(fx) * 0.9642, inv(fy), inv(fz) * 0.8249);
    // XYZ (D50) to linear sRGB, Bradford-adapted matrix.
    let r = 3.133_856_1 * x - 1.616_866_7 * y - 0.490_614_6 * z;
    let g = -0.978_768_4 * x + 1.916_141_5 * y + 0.033_454 * z;
    let bl = 0.071_945_3 * x - 0.228_991_4 * y + 1.405_242_7 * z;
    (encode(r) * 255.0, encode(g) * 255.0, encode(bl) * 255.0)
}

/// Oklab to sRGB.
pub(super) fn oklab_to_rgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let (r, g, bl) = oklab_to_linear(l, a, b);
    (encode(r) * 255.0, encode(g) * 255.0, encode(bl) * 255.0)
}

/// Oklab to linear sRGB.
pub(crate) fn oklab_to_linear(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let lp = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let mp = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let sp = l - 0.089_484_177_5 * a - 1.291_485_548 * b;
    let (l3, m3, s3) = (lp * lp * lp, mp * mp * mp, sp * sp * sp);
    (
        4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3,
        -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3,
        -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701 * s3,
    )
}

/// Linear sRGB to Oklab.
pub(crate) fn linear_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    (
        0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
    )
}

/// Encode one linear-light sRGB channel.
pub(crate) fn encode(u: f64) -> f64 {
    if u <= 0.003_130_8 {
        12.92 * u
    } else {
        1.055 * u.powf(1.0 / 2.4) - 0.055
    }
}

/// Decode one encoded sRGB channel to linear light.
pub(crate) fn decode(u: f64) -> f64 {
    if u <= 0.040_45 {
        u / 12.92
    } else {
        ((u + 0.055) / 1.055).powf(2.4)
    }
}
