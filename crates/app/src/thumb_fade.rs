//! Touch scroll thumbs as Chrome on Android draws them (cc's
//! `ANDROID_OVERLAY` scrollbar animator, `cc/input/
//! scrollbar_animation_controller.cc`, read at chromium 1555c8bf): each
//! scroll update shows the thumbs at full opacity at once (`Show`, no
//! fade in) and restarts a 300 ms delay, after which they fade out
//! linearly over 300 ms (`PostDelayedAnimation(kFadeOut)`,
//! `RunAnimationFrame`). The timings are `InitializeScrollbarFadeAndDelay`'s
//! (`third_party/blink/renderer/platform/widget/compositing/
//! layer_tree_settings.cc`). Chromium times the fade from the first
//! frame after the delay (`last_awaken_time_`); this times it from the
//! delay's end, which a frame drawn at once makes the same.

use std::time::Duration;

use vapors_platform::Instant;

/// `scrollbar_fade_delay` on Android.
pub(crate) const FADE_DELAY: Duration = Duration::from_millis(300);
/// `scrollbar_fade_duration` on Android.
pub(crate) const FADE_DURATION: Duration = Duration::from_millis(300);

/// Where the thumbs are in their fade at a time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Phase {
    /// Never shown, or faded out: nothing to draw or wait for.
    Hidden,
    /// Shown at full opacity until the fade starts at this time.
    Shown(Instant),
    /// Fading: a frame is wanted at once.
    Fading,
}

/// The last scroll update, which the fade counts from.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ThumbFade {
    last_scroll: Option<Instant>,
}

impl ThumbFade {
    /// A container scrolled at `now` (`DidScrollUpdate`).
    pub(crate) fn scrolled(&mut self, now: Instant) {
        self.last_scroll = Some(now);
    }

    /// The thumbs' opacity at `now`, 0 to 1.
    pub(crate) fn opacity(&self, now: Instant) -> f32 {
        let Some(fade_start) = self.fade_start() else {
            return 0.0;
        };
        let into = now.saturating_duration_since(fade_start);
        if now < fade_start {
            return 1.0;
        }
        (1.0 - into.as_secs_f32() / FADE_DURATION.as_secs_f32()).max(0.0)
    }

    /// Where the fade is at `now`.
    pub(crate) fn phase(&self, now: Instant) -> Phase {
        match self.fade_start() {
            Some(start) if now < start => Phase::Shown(start),
            Some(start) if now < start + FADE_DURATION => Phase::Fading,
            // never shown, or faded out
            _ => Phase::Hidden,
        }
    }

    fn fade_start(&self) -> Option<Instant> {
        self.last_scroll.map(|at| at + FADE_DELAY)
    }
}

#[cfg(test)]
#[path = "thumb_fade_tests.rs"]
mod tests;
