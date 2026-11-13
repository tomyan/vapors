//! SVG path data in the subset docs/icons.md lists (`M L H V C Q A Z`,
//! absolute and relative, with implicit repeats), read strictly into
//! absolute [`Segment`]s, and from those zeno's commands for the icon
//! route. Anything else is refused at the first character not
//! understood, never drawn approximately. The smooth curves `S` and `T`
//! come with docs/plans/svg-support.md V1.

use zeno::{Command, Point};

use crate::path_lex::Cursor;
pub use crate::path_lex::PathError;
use crate::segment::{Segment, to_zeno};

/// Path data that parsed: its segments, absolute, in viewBox units,
/// and zeno's commands for them.
#[derive(Clone, Debug, PartialEq)]
pub struct IconPath {
    segments: Vec<Segment>,
    commands: Vec<Command>,
}

impl IconPath {
    /// Reads `data`, an SVG `d` attribute in the subset.
    ///
    /// # Errors
    /// [`PathError`] at the first character not understood: empty data,
    /// data not starting with a move, a command outside `M L H V C Q A Z`,
    /// or a missing or malformed number or arc flag.
    pub fn parse(data: &str) -> Result<Self, PathError> {
        let mut cursor = Cursor::new(data);
        cursor.skip_space();
        if cursor.at_end() {
            return Err(
                cursor.error("path data is empty; write a move and a line, such as M1 1L5 5")
            );
        }
        if !matches!(cursor.peek(), Some(b'M' | b'm')) {
            return Err(cursor
                .error("path data must start with M or m; write M x y before the first command"));
        }
        let mut pen = Pen::default();
        while !cursor.at_end() {
            command(&mut cursor, &mut pen)?;
            cursor.skip_space();
        }
        let commands = to_zeno(&pen.segments);
        Ok(Self {
            segments: pen.segments,
            commands,
        })
    }

    /// The segments, absolute, in viewBox units.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// zeno's commands for the segments (arcs as curves), in viewBox
    /// units.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}

/// The segments so far and where the pen is: the current point, which
/// relative commands add to, and the subpath's start, where a close
/// returns it.
#[derive(Default)]
struct Pen {
    segments: Vec<Segment>,
    current: Point,
    start: Point,
}

impl Pen {
    fn push(&mut self, segment: Segment) {
        self.current = match segment {
            Segment::Move(p) => {
                self.start = p;
                p
            }
            Segment::Line(p) | Segment::Quad(_, p) | Segment::Cubic(_, _, p) => p,
            Segment::Arc { to, .. } => to,
            Segment::Close => self.start,
        };
        self.segments.push(segment);
    }
}

/// A command letter of the subset.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Verb {
    Move,
    Line,
    Horizontal,
    Vertical,
    Cubic,
    Quadratic,
    Arc,
    Close,
}

impl Verb {
    /// The verb `letter` names and whether it is relative.
    fn of(letter: u8) -> Option<(Self, bool)> {
        let verb = match letter.to_ascii_uppercase() {
            b'M' => Self::Move,
            b'L' => Self::Line,
            b'H' => Self::Horizontal,
            b'V' => Self::Vertical,
            b'C' => Self::Cubic,
            b'Q' => Self::Quadratic,
            b'A' => Self::Arc,
            b'Z' => Self::Close,
            _ => return None,
        };
        Some((verb, letter.is_ascii_lowercase()))
    }
}

/// One command letter and its argument groups, repeated while numbers
/// follow (a move's repeats are lines).
fn command(cursor: &mut Cursor<'_>, pen: &mut Pen) -> Result<(), PathError> {
    let letter = cursor.peek().expect("not at the end");
    let (mut verb, relative) = Verb::of(letter).ok_or_else(|| cursor.error(unknown(cursor)))?;
    cursor.advance();
    if verb == Verb::Close {
        pen.push(Segment::Close);
        return Ok(());
    }
    cursor.refuse_comma_after_command()?;
    loop {
        let from = if relative { pen.current } else { Point::ZERO };
        pen.push(group(cursor, verb, from, pen.current)?);
        if verb == Verb::Move {
            verb = Verb::Line;
        }
        if !cursor.number_follows() {
            return Ok(());
        }
    }
}

/// Why the character at the cursor is not a command of the subset.
fn unknown(cursor: &Cursor<'_>) -> String {
    let found = cursor.found();
    if matches!(cursor.peek(), Some(b'S' | b's' | b'T' | b't')) {
        return format!(
            "{found} (a smooth curve) is not in the subset, which draws M L H V C Q A Z; write it as C or Q with the reflected control point"
        );
    }
    format!(
        "expected a command, one of M L H V C Q A Z (lower case for relative), found {found}; write the segment with those commands"
    )
}

/// One argument group of `verb`: its points are added to `from` (the
/// current point when the command is relative, else zero); `current`
/// supplies the coordinate an `H` or `V` keeps.
fn group(
    cursor: &mut Cursor<'_>,
    verb: Verb,
    from: Point,
    current: Point,
) -> Result<Segment, PathError> {
    Ok(match verb {
        Verb::Move => Segment::Move(point(cursor)? + from),
        Verb::Line => Segment::Line(point(cursor)? + from),
        Verb::Horizontal => Segment::Line(Point::new(cursor.number()? + from.x, current.y)),
        Verb::Vertical => Segment::Line(Point::new(current.x, cursor.number()? + from.y)),
        Verb::Quadratic => Segment::Quad(point(cursor)? + from, point(cursor)? + from),
        Verb::Cubic => Segment::Cubic(
            point(cursor)? + from,
            point(cursor)? + from,
            point(cursor)? + from,
        ),
        Verb::Arc => arc(cursor, from)?,
        Verb::Close => unreachable!("a close takes no arguments"),
    })
}

fn point(cursor: &mut Cursor<'_>) -> Result<Point, PathError> {
    Ok(Point::new(cursor.number()?, cursor.number()?))
}

/// An arc: radii, rotation, the two flags and the end point.
fn arc(cursor: &mut Cursor<'_>, from: Point) -> Result<Segment, PathError> {
    let radii = [cursor.number()?, cursor.number()?];
    let rotation = cursor.number()?;
    let (large, sweep) = (cursor.flag()?, cursor.flag()?);
    let to = point(cursor)? + from;
    Ok(Segment::Arc {
        radii,
        rotation,
        large,
        sweep,
        to,
    })
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "path_arc_tests.rs"]
mod arc_tests;

#[cfg(test)]
#[path = "path_curve_tests.rs"]
mod curve_tests;

#[cfg(test)]
#[path = "path_refusals_tests.rs"]
mod refusals_tests;
