//! The events oracle: each fixture with an `<name>.events` beside its
//! `.wisp` is opened in a live headless Chromium, its script played
//! (`events_play`), and the answers written to or compared with
//! `fixtures/<name>.events.json`. An `.events` with no `.wisp` is refused.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;
use vapors_devtools::Session;

use crate::events_play::play;
use crate::events_script::parse;
use crate::oracle::fixtures_dir;

/// vapors' `COLD_START`, 145 s, about 2.5x the slowest first launch on a
/// runner (57.9 s); a warm browser answers in milliseconds.
const DEADLINE: Duration = Duration::from_secs(145);

/// The names of the fixtures with a script, sorted.
///
/// # Errors
/// The directory cannot be read, or a script has no fixture.
pub fn scripted() -> Result<Vec<String>, String> {
    let dir = fixtures_dir();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let file = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = file.strip_suffix(".events") else {
            continue;
        };
        if !dir.join(format!("{name}.wisp")).is_file() {
            return Err(format!(
                "{}: no {name}.wisp beside it; fix: add the fixture or remove the script",
                dir.join(&file).display()
            ));
        }
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

/// Fixture `name`'s answers to its script, every expectation held.
///
/// # Errors
/// A script of another shape, a page that does not open, or an
/// expectation that does not hold, each naming its line.
pub fn answers(session: &mut Session, work: &Path, name: &str) -> Result<Value, String> {
    let path = fixtures_dir().join(format!("{name}.events"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let script = parse(name, &text)?;
    crate::ax::open(session, work, name, script.viewport)?;
    play(session, name, &script, DEADLINE)
}
