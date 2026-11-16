//! A fling transcript from scripts/fling-recorder and the coast fitted
//! to its first frames beside it (`fit.py coast`; docs/plans/
//! touch-momentum.md): the page's density and list, where and how fast
//! Chrome's coast began, and Chrome's frames from the coast's first on.

#![allow(
    clippy::cast_possible_truncation,
    reason = "a transcript's numbers are a phone page's CSS pixels and milliseconds, far inside f32"
)]

use std::path::Path;

use serde_json::Value;

/// The recorder's list: its size and its content's height, CSS px.
pub struct List {
    pub width: f32,
    pub height: f32,
    pub scroll_height: f32,
}

/// Where and how fast Chrome's coast began, and its first frame, ms on
/// the transcript's clock and CSS px.
pub struct Coast {
    pub first_frame_ms: f64,
    pub start_ms: f64,
    pub start_top: f32,
    pub velocity: f32,
    /// How many of the coast's first frames the fit saw.
    pub fit_frames: f64,
}

/// A transcript's coast.
pub struct Transcript {
    pub file: String,
    pub scale: f32,
    pub list: List,
    pub coast: Coast,
    /// Chrome's frames from the coast's first: time in ms, `scrollTop`.
    pub frames: Vec<(f64, f32)>,
}

fn json(path: &Path) -> Result<Value, String> {
    let file = path.display();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{file}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{file}: not JSON: {e}"))
}

fn number(file: &str, value: &Value, name: &str) -> Result<f64, String> {
    value[name]
        .as_f64()
        .ok_or_else(|| format!("{file}: `{name}` is not a number"))
}

/// The transcript in `path` and its coast beside it
/// (`<name>.coast.json`).
///
/// # Errors
/// A file that does not read or parse, a field missing or of another
/// shape, or an event of a kind the recorder does not write, naming the
/// file and the field or event.
pub fn load(path: &Path) -> Result<Transcript, String> {
    let file = path.display().to_string();
    let (recorded, fitted) = (json(path)?, json(&path.with_extension("coast.json"))?);
    let coast = coast(&file, &fitted)?;
    let mut frames = frames(&file, &recorded)?;
    frames.retain(|(t, _)| *t >= coast.first_frame_ms);
    Ok(Transcript {
        scale: number(&file, &recorded, "devicePixelRatio")? as f32,
        list: list(&file, &recorded["list"])?,
        coast,
        frames,
        file,
    })
}

fn coast(file: &str, fitted: &Value) -> Result<Coast, String> {
    Ok(Coast {
        first_frame_ms: number(file, fitted, "first_frame_ms")?,
        start_ms: number(file, fitted, "start_ms")?,
        start_top: number(file, fitted, "start_top")? as f32,
        velocity: number(file, fitted, "velocity")? as f32,
        fit_frames: number(file, fitted, "fit_frames")?,
    })
}

fn list(file: &str, list: &Value) -> Result<List, String> {
    Ok(List {
        width: number(file, list, "width")? as f32,
        height: number(file, list, "height")? as f32,
        scroll_height: number(file, list, "scrollHeight")? as f32,
    })
}

/// Every frame in the transcript's events.
fn frames(file: &str, recorded: &Value) -> Result<Vec<(f64, f32)>, String> {
    let events = recorded["events"]
        .as_array()
        .ok_or_else(|| format!("{file}: no `events` list"))?;
    let mut frames = Vec::new();
    for (at, event) in events.iter().enumerate() {
        if let Some(frame) =
            recorded_frame(event).map_err(|e| format!("{file}: event {at}: {e}"))?
        {
            frames.push(frame);
        }
    }
    Ok(frames)
}

/// `event`'s time and `scrollTop` when it is a frame; `None` for the
/// other kinds the recorder writes.
fn recorded_frame(event: &Value) -> Result<Option<(f64, f32)>, String> {
    let number = |name: &str| {
        event[name]
            .as_f64()
            .ok_or(format!("`{name}` is not a number"))
    };
    match event["kind"].as_str().ok_or("no `kind`")? {
        "frame" => Ok(Some((number("t")?, number("top")? as f32))),
        "scrollend" | "touchstart" | "touchmove" | "touchend" | "touchcancel" => Ok(None),
        other => Err(format!("unknown kind `{other}`")),
    }
}
