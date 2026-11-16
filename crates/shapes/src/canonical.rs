//! A shape's canonical form (docs/plans/one-scene.md 1.3): the bytes
//! that make two shapes one table entry, and their hash. The parsed
//! segments, absolute in viewBox units, after the paint's fill rule or
//! the stroke parameters that change the mesh. Paint colour is per
//! item and not here.
//!
//! The form folds the parser's rewrites (relative commands, `H` and
//! `V`, implicit repeats, the arc corrections, `-0`) and nothing
//! drawn-equal beyond them: `M0.3 0.1l0.6 0` and `M0.3 0.1L0.9 0.1`
//! are two entries (0.3 + 0.6 is not 0.9 in f32), as are a filled
//! open subpath and its closed twin. Each costs an entry, never a
//! wrong draw. Equality is per build target: a scaled, rotated arc's
//! radii pass through `sin_cos`, whose last bit differs between the
//! host's libm and wasm32's, so a run-time parse (svg-support S4a)
//! compares bytes on a hash hit and takes a miss as a cache miss. A
//! change to the verb bytes, the arc encoding or the paint prefix
//! bumps [`crate::TABLE_FORMAT`].

use zeno::Point;

use crate::path::IconPath;
use crate::segment::Segment;

/// How a shape is painted, as far as it changes the geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    /// The path's interior under `rule`.
    Fill(FillRule),
    /// The path's outline, `width` wide, with SVG's default miter joins
    /// at limit 4 and butt caps (other joins and caps come with
    /// docs/plans/svg-support.md V3).
    Stroke {
        /// `stroke-width`, in viewBox units, above zero.
        width: f32,
    },
}

/// SVG's `fill-rule`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillRule {
    /// Inside where the winding number is not zero.
    NonZero,
    /// Inside where the crossing count is odd.
    EvenOdd,
}

/// A shape's canonical bytes and their hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canonical {
    bytes: Vec<u8>,
    hash: u64,
}

impl Canonical {
    /// The canonical form of `path` painted by `paint`; `site` names
    /// the shape in a refusal.
    ///
    /// # Errors
    /// A stroke width that is not a finite number above zero, naming
    /// the site and the width.
    pub fn of(site: &str, path: &IconPath, paint: Paint) -> Result<Self, String> {
        let mut bytes = Vec::with_capacity(8 + path.segments().len() * 9);
        write_paint(site, paint, &mut bytes)?;
        for segment in path.segments() {
            write_segment(segment, &mut bytes);
        }
        let hash = hash_bytes(&bytes);
        Ok(Self { bytes, hash })
    }

    /// The bytes; equal bytes are one shape.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// [`hash_bytes`] of the bytes.
    #[must_use]
    pub const fn hash(&self) -> u64 {
        self.hash
    }
}

/// The hash a table entry carries for its canonical bytes: FNV-1a,
/// 64 bits, written out here so the compiler's build and the page's
/// run agree on every target (a hasher keyed by `usize` would not on
/// wasm32).
#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

/// The paint canonical `bytes` begin with, as [`Canonical::of`] wrote
/// it: what a closed form is drawn with, read back from the one place
/// that holds it.
///
/// # Errors
/// Bytes that do not begin with a paint prefix (`F`, `E`, or `S` and a
/// finite width above zero), naming the first byte.
pub fn paint_of(bytes: &[u8]) -> Result<Paint, String> {
    match bytes {
        [b'F', ..] => Ok(Paint::Fill(FillRule::NonZero)),
        [b'E', ..] => Ok(Paint::Fill(FillRule::EvenOdd)),
        [b'S', a, b, c, d, ..] => {
            let width = f32::from_le_bytes([*a, *b, *c, *d]);
            if width.is_finite() && width > 0.0 {
                Ok(Paint::Stroke { width })
            } else {
                Err(format!(
                    "the canonical bytes stroke at width {width}, not a finite number above zero"
                ))
            }
        }
        _ => Err(format!(
            "the canonical bytes begin {:?}, not a paint (F, E, or S and a width)",
            bytes.get(..5.min(bytes.len())).unwrap_or_default()
        )),
    }
}

fn write_paint(site: &str, paint: Paint, out: &mut Vec<u8>) -> Result<(), String> {
    match paint {
        Paint::Fill(FillRule::NonZero) => out.push(b'F'),
        Paint::Fill(FillRule::EvenOdd) => out.push(b'E'),
        Paint::Stroke { width } => {
            if !(width.is_finite() && width > 0.0) {
                return Err(format!(
                    "the shape at {site} has stroke-width {width}, which is not a finite number above zero; write a positive width"
                ));
            }
            out.push(b'S');
            write_number(width, out);
        }
    }
    Ok(())
}

fn write_segment(segment: &Segment, out: &mut Vec<u8>) {
    match *segment {
        Segment::Move(p) => write_points(b'M', &[p], out),
        Segment::Line(p) => write_points(b'L', &[p], out),
        Segment::Quad(c, p) => write_points(b'Q', &[c, p], out),
        Segment::Cubic(a, b, p) => write_points(b'C', &[a, b, p], out),
        Segment::Arc {
            radii,
            rotation,
            large,
            sweep,
            to,
        } => write_arc(radii, rotation, [large, sweep], to, out),
        Segment::Close => out.push(b'Z'),
    }
}

/// `A`, the radii, the rotation, the two flags in one byte (large in
/// bit 0, sweep in bit 1), then `>` and the end point.
fn write_arc(
    radii: [f32; 2],
    rotation: f32,
    [large, sweep]: [bool; 2],
    to: Point,
    out: &mut Vec<u8>,
) {
    out.push(b'A');
    write_number(radii[0], out);
    write_number(radii[1], out);
    write_number(rotation, out);
    out.push(u8::from(large) | (u8::from(sweep) << 1));
    write_points(b'>', &[to], out);
}

fn write_points(verb: u8, points: &[Point], out: &mut Vec<u8>) {
    out.push(verb);
    for point in points {
        write_number(point.x, out);
        write_number(point.y, out);
    }
}

/// A finite number's bits, little-endian. The parser refuses a
/// non-finite coordinate and the lexer reads `-0` as `0`, so `M-0 0`
/// and `M0 0` arrive as one shape; both are asserted here so the two
/// modules stay tied.
fn write_number(value: f32, out: &mut Vec<u8>) {
    debug_assert!(
        value.is_finite(),
        "{value} reached the canonical form; the parser refuses it"
    );
    debug_assert!(
        value != 0.0 || value.to_bits() == 0,
        "-0 reached the canonical form; the lexer reads it as 0"
    );
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
#[path = "canonical_tests.rs"]
mod tests;
