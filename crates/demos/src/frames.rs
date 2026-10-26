//! A per-frame demo held to a number of frames, for a headless page that
//! must go idle before its screenshot: `?demo=stress&ticks=5`.

use std::time::Duration;

use crate::PerFrame;

/// `per_frame`, which does nothing after its first `frames` calls.
pub fn limited(mut per_frame: PerFrame, frames: usize) -> PerFrame {
    let mut left = frames;
    Box::new(move |last: &[(&'static str, Duration)]| {
        if left > 0 {
            left -= 1;
            per_frame(last);
        }
    })
}

/// The `ticks` query value as a number of frames: `None` when absent.
///
/// # Errors
/// A value that is not a whole number, naming it.
pub fn frame_limit(ticks: Option<&str>) -> Result<Option<usize>, String> {
    ticks
        .map(|text| {
            text.parse()
                .map_err(|_| format!("ticks={text} is not a whole number of frames; give one, such as ticks=5, or leave it out to tick forever"))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn a_limited_hook_runs_its_first_frames_and_then_nothing() {
        // Given
        let calls = Rc::new(Cell::new(0));
        let counted = Rc::clone(&calls);
        let mut hook = limited(Box::new(move |_| counted.set(counted.get() + 1)), 3);

        // When
        for _ in 0..5 {
            hook(&[]);
        }

        // Then
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn ticks_is_a_whole_number_or_absent_and_anything_else_is_refused() {
        // Then
        assert_eq!(frame_limit(None), Ok(None));
        assert_eq!(frame_limit(Some("5")), Ok(Some(5)));
        let refused = frame_limit(Some("five")).expect_err("refused");
        assert!(refused.contains("ticks=five"), "{refused}");
    }
}
