//! The walk's run directory (`ux.run_dir`): one directory per viewport,
//! each with `facts.json`, whose `steps` name the frames and texts beside
//! it. Anything else in the run directory is refused, so a review never
//! judges a frame the walk did not record.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// One frame the walk saved, with its size in pixels.
#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
    /// `<viewport>/<nn>-<step>.png`, as the report cites it.
    pub name: String,
    pub path: PathBuf,
    pub size: (u32, u32),
    /// The step's visible text, from its `.txt`.
    pub text: String,
}

/// One viewport's facts and frames.
#[derive(Debug)]
pub struct Viewport {
    pub name: String,
    pub facts: Value,
    pub frames: Vec<Frame>,
}

/// Read every viewport under `dir`, by name.
///
/// # Errors
/// A missing or empty run directory, a subdirectory or file that is not
/// a viewport, or a viewport whose facts or frames cannot be read.
pub fn read(dir: &Path) -> Result<Vec<Viewport>, String> {
    let fix = "fix: run the walk first";
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}; {fix}", dir.display()))?;
    let mut viewports = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if !path.join("facts.json").is_file() {
            return Err(format!(
                "{} is not a viewport (no facts.json); the run directory holds only viewports",
                path.display()
            ));
        }
        viewports.push(viewport(&path)?);
    }
    if viewports.is_empty() {
        return Err(format!("{} holds no viewports; {fix}", dir.display()));
    }
    viewports.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(viewports)
}

fn viewport(dir: &Path) -> Result<Viewport, String> {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("{}: not a UTF-8 name", dir.display()))?
        .to_owned();
    let facts_path = dir.join("facts.json");
    let text = std::fs::read_to_string(&facts_path)
        .map_err(|e| format!("{}: {e}", facts_path.display()))?;
    let facts: Value = serde_json::from_str(&text)
        .map_err(|e| format!("{}: not JSON: {e}", facts_path.display()))?;
    let steps = facts["steps"]
        .as_array()
        .ok_or_else(|| format!("{}: no `steps` list", facts_path.display()))?;
    let frames = steps
        .iter()
        .map(|step| frame(dir, &name, step))
        .collect::<Result<Vec<_>, _>>()?;
    if frames.is_empty() {
        return Err(format!("{}: `steps` is empty", facts_path.display()));
    }
    Ok(Viewport {
        name,
        facts,
        frames,
    })
}

fn frame(dir: &Path, viewport: &str, step: &Value) -> Result<Frame, String> {
    let named = |key: &str, extension: &str| {
        step[key]
            .as_str()
            .filter(|f| plain_name(f, extension))
            .ok_or_else(|| {
                format!(
                    "{}/facts.json: a step's `{key}` is not a plain `{extension}` file name beside it: {step}",
                    dir.display()
                )
            })
    };
    let (png_file, text_file) = (named("frame", ".png")?, named("text", ".txt")?);
    let path = dir.join(png_file);
    let text = std::fs::read_to_string(dir.join(text_file))
        .map_err(|e| format!("{}/{text_file}: {e}", dir.display()))?;
    Ok(Frame {
        name: format!("{viewport}/{png_file}"),
        size: size(&path)?,
        path,
        text,
    })
}

/// A file name with a stem and `extension`, and no path in it.
fn plain_name(name: &str, extension: &str) -> bool {
    name.strip_suffix(extension)
        .is_some_and(|stem| !stem.is_empty() && !stem.starts_with('.'))
        && !name.contains(['/', '\\'])
}

/// The keys of facts.json that are its structure, not facts: a measured
/// checklist item cannot name one.
pub const STRUCTURE: [&str; 3] = ["viewport", "size", "steps"];

/// A PNG's width and height, from its header.
fn size(path: &Path) -> Result<(u32, u32), String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let reader = png::Decoder::new(BufReader::new(file))
        .read_info()
        .map_err(|e| format!("{} is not a PNG this tool can read: {e}", path.display()))?;
    let info = reader.info();
    Ok((info.width, info.height))
}

#[cfg(test)]
#[path = "ux_run_tests.rs"]
mod tests;
