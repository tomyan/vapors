//! SVG path data in the subset docs/icons.md lists (`M L H V C Q A Z`,
//! absolute and relative, with implicit repeats), read strictly into
//! absolute [`Segment`]s, and from those zeno's commands for the icon
//! route. Anything else is refused at the first character not
//! understood, never drawn approximately. The smooth curves `S` and `T`
//! come with docs/plans/svg-support.md V1.

use zeno::{Command, PathBuilder, Point};

use crate::path_lex::Cursor;
pub use crate::path_lex::PathError;
use crate::path_verb::Verb;
use crate::segment::{Pen, Segment, non_finite, to_zeno};

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
    /// a missing or malformed number or arc flag, or a relative command
    /// whose coordinate overflows to infinity.
    ///
    /// # Panics
    /// When the segments and zeno's commands end at different points:
    /// a bug in the arc corrections, which `Pen::push` applies once for
    /// both (every coordinate is finite by then, so the comparison is).
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
        let commands = to_zeno(pen.segments());
        assert!(
            agree(commands.current_point(), pen.current()),
            "{data:?}: the segments end at {:?} and zeno's commands at {:?}; the arc corrections in Pen::push and to_zeno disagree",
            pen.current(),
            commands.current_point()
        );
        Ok(Self {
            segments: pen.into_segments(),
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

/// Whether the pen's end and zeno's agree: zeno computes an arc's end
/// from its curves, within rounding of the end point written.
fn agree(a: Point, b: Point) -> bool {
    let scale = a.x.abs().max(a.y.abs()).max(1.0);
    (a.x - b.x).abs() <= 1e-3 * scale && (a.y - b.y).abs() <= 1e-3 * scale
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
        let from = if relative { pen.current() } else { Point::ZERO };
        let segment = group(cursor, verb, from, pen.current())?;
        if let Some(value) = non_finite(&segment) {
            return Err(cursor.error(format!(
                "a coordinate of this {} is {value} after adding the current point {:?}; a shape's numbers must stay under 3.4e38, so write it in absolute coordinates or nearer the origin",
                letter as char,
                pen.current()
            )));
        }
        pen.push(segment);
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
#[path = "path_pen_tests.rs"]
mod pen_tests;

#[cfg(test)]
#[path = "path_refusals_tests.rs"]
mod refusals_tests;
