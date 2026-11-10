//! A failing `--check` prints this: each frame's decision and each
//! surface's flags, and the asks the app acted on, with runs of
//! identical frames collapsed, so a failure seen only in CI's log can
//! be diagnosed from it.

use std::fmt::Write;

use vapors_app::{FrameDecision, SurfaceState};
use vapors_platform::Next;

/// One line of the report: frames `first..=last` that looked alike, or
/// a note at `first`.
struct Entry {
    first: u64,
    last: u64,
    text: String,
    is_frame: bool,
}

/// What the app did, frame by frame.
#[derive(Default)]
pub struct Trace {
    entries: Vec<Entry>,
}

impl Trace {
    /// Frame `number` ended asking for `next` for the reasons in `decision`.
    pub fn frame(
        &mut self,
        number: u64,
        next: Next,
        decision: FrameDecision,
        states: &[SurfaceState],
    ) {
        let text = format!(
            "next {next:?}; take_changed {}, any_stale {}, surfaces_want {}; {}",
            decision.document_changed,
            decision.surface_invalidated,
            decision.surfaces_want,
            surfaces(states)
        );
        match self.entries.last_mut() {
            Some(last) if last.is_frame && last.text == text => last.last = number,
            _ => self.push(number, text, true),
        }
    }

    /// Something that happened after frame `number`.
    pub fn note(&mut self, number: u64, text: String) {
        self.push(number, text, false);
    }

    fn push(&mut self, number: u64, text: String, is_frame: bool) {
        self.entries.push(Entry {
            first: number,
            last: number,
            text,
            is_frame,
        });
    }

    /// One line per run, oldest first.
    pub fn report(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            let (first, last, text) = (entry.first, entry.last, &entry.text);
            let _ = if !entry.is_frame {
                writeln!(out, "  after frame {first}: {text}")
            } else if first == last {
                writeln!(out, "  frame {first}: {text}")
            } else {
                writeln!(out, "  frames {first}..={last}: {text}")
            };
        }
        out
    }
}

/// Each surface as `name[fresh stale animating backing waiting]`, with
/// only the flags that hold.
fn surfaces(states: &[SurfaceState]) -> String {
    let each = states.iter().map(|state| {
        let flags = [
            (state.fresh, "fresh"),
            (state.stale, "stale"),
            (state.animating, "animating"),
            (state.backing, "backing"),
            (state.waiting, "waiting"),
        ];
        let held: Vec<_> = flags
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, f)| *f)
            .collect();
        format!("{}[{}]", state.name, held.join(" "))
    });
    each.collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(animating: bool) -> SurfaceState {
        SurfaceState {
            name: "gradient".into(),
            fresh: false,
            stale: false,
            animating,
            backing: true,
            waiting: false,
        }
    }

    #[test]
    fn identical_frames_collapse_into_one_line_and_a_change_starts_another() {
        // Given
        let mut trace = Trace::default();
        let wants = FrameDecision {
            surfaces_want: true,
            ..FrameDecision::default()
        };

        // When
        for number in 5..=9 {
            trace.frame(number, Next::Continue, wants, &[state(true)]);
        }
        trace.note(9, "StopAnimation".into());
        trace.frame(10, Next::Wait, FrameDecision::default(), &[state(false)]);

        // Then
        let lines: Vec<_> = trace.report().lines().map(str::to_owned).collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("  frames 5..=9: next Continue; take_changed false, any_stale false, surfaces_want true; gradient[animating backing]"));
        assert_eq!(lines[1], "  after frame 9: StopAnimation");
        assert!(lines[2].starts_with("  frame 10: next Wait;"));
    }
}
