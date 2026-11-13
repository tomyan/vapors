//! A shape's canonical form (docs/plans/one-scene.md 1.3): the bytes
//! that make two shapes one table entry, and their hash. The parsed
//! segments, absolute in viewBox units, after the paint's fill rule or
//! the stroke parameters that change the mesh. Paint colour is per
//! item and not here.

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
        } => {
            out.push(b'A');
            write_number(radii[0], out);
            write_number(radii[1], out);
            write_number(rotation, out);
            out.push(u8::from(large) | (u8::from(sweep) << 1));
            write_points(b'>', &[to], out);
        }
        Segment::Close => out.push(b'Z'),
    }
}

fn write_points(verb: u8, points: &[Point], out: &mut Vec<u8>) {
    out.push(verb);
    for point in points {
        write_number(point.x, out);
        write_number(point.y, out);
    }
}

/// A finite number's bits, little-endian. The lexer reads `-0` as `0`,
/// so `M-0 0` and `M0 0` arrive as one shape.
fn write_number(value: f32, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
#[path = "canonical_tests.rs"]
mod tests;
