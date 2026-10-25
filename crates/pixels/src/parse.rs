//! Reading a `.pixels` file: its viewport sections, inputs and checks.

/// One check.
#[derive(Clone, Debug, PartialEq)]
pub enum Check {
    /// The pixel at (x, y) is this colour.
    Pixel {
        /// CSS pixels from the left.
        x: u32,
        /// CSS pixels from the top.
        y: u32,
        /// The colour.
        rgb: [u8; 3],
    },
    /// Some pixel in the box differs from this colour: something was drawn.
    Ink {
        /// Top-left corner, included.
        from: [u32; 2],
        /// Bottom-right corner, excluded.
        to: [u32; 2],
        /// The colour the box would be with nothing drawn on it.
        rgb: [u8; 3],
    },
}

/// One input made before the screenshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// A click at (x, y), CSS pixels.
    Click([u32; 2]),
    /// A wheel scroll at `at` by `delta` CSS pixels, signed as the DOM's
    /// `deltaX` and `deltaY`: positive scrolls right and down.
    Wheel {
        /// Where, CSS pixels.
        at: [u32; 2],
        /// By how much, CSS pixels.
        delta: [i32; 2],
    },
}

impl Input {
    /// The input as its `.pixels` line, which `check_pixels --inputs`
    /// prints for the page-contract script.
    #[must_use]
    pub fn line(&self) -> String {
        match self {
            Input::Click([x, y]) => format!("click {x} {y}"),
            Input::Wheel {
                at: [x, y],
                delta: [dx, dy],
            } => format!("wheel {x} {y} {dx} {dy}"),
        }
    }
}

/// The viewport a section with no `at` line is for.
pub const DEFAULT_VIEWPORT: [u32; 2] = [640, 400];

/// One section of a `.pixels` file: a viewport, the inputs to make, then
/// the checks to hold.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    /// The viewport, CSS pixels.
    pub viewport: [u32; 2],
    /// Clicks and wheel scrolls to make before the screenshot, in order.
    pub inputs: Vec<Input>,
    /// What the screenshot must show after them.
    pub checks: Vec<Check>,
}

impl Spec {
    fn new(viewport: [u32; 2]) -> Self {
        Self {
            viewport,
            inputs: Vec::new(),
            checks: Vec::new(),
        }
    }
}

enum Line {
    At([u32; 2]),
    Input(Input),
    Check(Check),
}

/// The sections of a `.pixels` file, the default viewport's first.
///
/// # Errors
/// A line that is not one of the four shapes, naming the line, or two
/// sections for one viewport.
pub fn parse(name: &str, text: &str) -> Result<Vec<Spec>, String> {
    let (mut done, mut current) = (Vec::new(), Spec::new(DEFAULT_VIEWPORT));
    let lines = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'));
    for (index, line) in lines {
        let at = |e: String| format!("{name}.pixels:{}: {e}: {line}", index + 1);
        match parse_line(line).map_err(at)? {
            Line::At(viewport) => {
                let seen = current.viewport == viewport
                    || done.iter().any(|s: &Spec| s.viewport == viewport);
                if seen {
                    return Err(at(format!(
                        "a second section for {}x{}",
                        viewport[0], viewport[1]
                    )));
                }
                done.push(std::mem::replace(&mut current, Spec::new(viewport)));
            }
            Line::Input(input) => current.inputs.push(input),
            Line::Check(check) => current.checks.push(check),
        }
    }
    done.push(current);
    Ok(done)
}

/// The section for `viewport`.
///
/// # Errors
/// When the file has none, naming the sizes it has.
pub fn section(specs: &[Spec], viewport: [u32; 2]) -> Result<&Spec, String> {
    specs
        .iter()
        .find(|s| s.viewport == viewport)
        .ok_or_else(|| {
            let sizes: Vec<String> = specs
                .iter()
                .map(|s| format!("{}x{}", s.viewport[0], s.viewport[1]))
                .collect();
            format!(
                "no section for {}x{}; the file has {}",
                viewport[0],
                viewport[1],
                sizes.join(", ")
            )
        })
}

fn parse_line(line: &str) -> Result<Line, String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    match fields.as_slice() {
        ["at", size, ..] => {
            let (w, h) = size.split_once('x').ok_or(format!("`{size}` is not WxH"))?;
            Ok(Line::At([number(w)?, number(h)?]))
        }
        ["click", x, y, ..] => Ok(Line::Input(Input::Click([number(x)?, number(y)?]))),
        ["wheel", x, y, dx, dy, ..] => Ok(Line::Input(Input::Wheel {
            at: [number(x)?, number(y)?],
            delta: [delta(dx)?, delta(dy)?],
        })),
        _ => parse_check(&fields).map(Line::Check),
    }
}

fn parse_check(fields: &[&str]) -> Result<Check, String> {
    match fields {
        ["pixel", x, y, rgb, ..] => Ok(Check::Pixel {
            x: number(x)?,
            y: number(y)?,
            rgb: colour(rgb)?,
        }),
        ["ink", x0, y0, x1, y1, rgb, ..] => Ok(Check::Ink {
            from: [number(x0)?, number(y0)?],
            to: [number(x1)?, number(y1)?],
            rgb: colour(rgb)?,
        }),
        _ => {
            Err("expected `click X Y`, `wheel X Y DX DY`, `pixel X Y #rrggbb` or `ink X0 Y0 X1 Y1 #rrggbb`".to_owned())
        }
    }
}

fn number(text: &str) -> Result<u32, String> {
    text.parse()
        .map_err(|_| format!("`{text}` is not a whole number of pixels"))
}

fn delta(text: &str) -> Result<i32, String> {
    text.parse()
        .map_err(|_| format!("`{text}` is not a whole number of pixels to scroll"))
}

fn colour(text: &str) -> Result<[u8; 3], String> {
    let hex = text
        .strip_prefix('#')
        .filter(|h| h.len() == 6)
        .ok_or(format!("`{text}` is not #rrggbb"))?;
    let channel = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| format!("`{text}` is not #rrggbb"))
    };
    Ok([channel(0)?, channel(2)?, channel(4)?])
}
