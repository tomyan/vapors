//! An icon's path: the subset of SVG path data docs/icons.md lists
//! (`M L H V A Z`, absolute and relative, with implicit repeats), read
//! strictly into zeno's commands. Anything else is refused at the first
//! character not understood, never drawn approximately.

use zeno::{Angle, ArcSize, ArcSweep, Command, PathBuilder, Point};

use crate::icon_path_lex::Cursor;
pub use crate::icon_path_lex::PathError;

/// Path data that parsed, as zeno commands in viewBox units.
#[derive(Clone, Debug, PartialEq)]
pub struct IconPath {
    commands: Vec<Command>,
}

impl IconPath {
    /// Reads `data`, an SVG `d` attribute in the icon subset.
    ///
    /// # Errors
    /// [`PathError`] at the first character not understood: empty data,
    /// data not starting with a move, a command outside `M L H V A Z`,
    /// or a missing or malformed number or arc flag.
    pub fn parse(data: &str) -> Result<Self, PathError> {
        let mut cursor = Cursor::new(data);
        cursor.skip_space();
        if cursor.at_end() {
            return Err(cursor.error("path data is empty; give d a move (M) and a line"));
        }
        if !matches!(cursor.peek(), Some(b'M' | b'm')) {
            return Err(cursor.error("path data must start with M or m"));
        }
        let mut commands = Vec::new();
        while !cursor.at_end() {
            segment(&mut cursor, &mut commands)?;
            cursor.skip_space();
        }
        Ok(Self { commands })
    }

    /// The commands, in viewBox units.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}

/// A command letter of the subset.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Verb {
    Move,
    Line,
    Horizontal,
    Vertical,
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
            b'A' => Self::Arc,
            b'Z' => Self::Close,
            _ => return None,
        };
        Some((verb, letter.is_ascii_lowercase()))
    }
}

/// One command letter and its argument groups, repeated while numbers
/// follow (a move's repeats are lines).
fn segment(cursor: &mut Cursor<'_>, out: &mut Vec<Command>) -> Result<(), PathError> {
    let letter = cursor.peek().expect("not at the end");
    let (mut verb, relative) = Verb::of(letter).ok_or_else(|| cursor.error(unknown(letter)))?;
    cursor.advance();
    if verb == Verb::Close {
        out.close();
        return Ok(());
    }
    loop {
        group(cursor, verb, relative, out)?;
        if verb == Verb::Move {
            verb = Verb::Line;
        }
        if !cursor.number_follows() {
            return Ok(());
        }
    }
}

fn unknown(letter: u8) -> String {
    let shown = char::from(letter);
    if b"CcSsQqTt".contains(&letter) {
        return format!(
            "{shown:?} (a curve) is not in the icon subset, which draws M L H V A Z (docs/icons.md); draw it with lines and arcs"
        );
    }
    format!("expected a command, one of M L H V A Z (or lower case for relative), found {shown:?}")
}

/// One argument group of `verb`, added to `out`.
fn group(
    cursor: &mut Cursor<'_>,
    verb: Verb,
    relative: bool,
    out: &mut Vec<Command>,
) -> Result<(), PathError> {
    let from = if relative {
        out.current_point()
    } else {
        Point::ZERO
    };
    match verb {
        Verb::Move => out.move_to(point(cursor)? + from),
        Verb::Line => out.line_to(point(cursor)? + from),
        Verb::Horizontal => axis_line(out, [Some(cursor.number()? + from.x), None]),
        Verb::Vertical => axis_line(out, [None, Some(cursor.number()? + from.y)]),
        Verb::Arc => arc(cursor, from, out)?,
        Verb::Close => unreachable!("a close takes no arguments"),
    };
    Ok(())
}

fn point(cursor: &mut Cursor<'_>) -> Result<Point, PathError> {
    Ok(Point::new(cursor.number()?, cursor.number()?))
}

/// A line to `[x, y]`, keeping the current coordinate where one is `None`.
fn axis_line(out: &mut Vec<Command>, [x, y]: [Option<f32>; 2]) -> &mut Vec<Command> {
    let current = out.current_point();
    out.line_to(Point::new(x.unwrap_or(current.x), y.unwrap_or(current.y)))
}

fn arc<'a>(
    cursor: &mut Cursor<'_>,
    from: Point,
    out: &'a mut Vec<Command>,
) -> Result<&'a mut Vec<Command>, PathError> {
    let (rx, ry, angle) = (cursor.number()?, cursor.number()?, cursor.number()?);
    let size = if cursor.flag()? {
        ArcSize::Large
    } else {
        ArcSize::Small
    };
    let sweep = if cursor.flag()? {
        ArcSweep::Positive
    } else {
        ArcSweep::Negative
    };
    let to = point(cursor)? + from;
    Ok(out.arc_to(rx, ry, Angle::from_degrees(angle), size, sweep, to))
}

#[cfg(test)]
#[path = "icon_path_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "icon_path_refusals_tests.rs"]
mod refusals_tests;
