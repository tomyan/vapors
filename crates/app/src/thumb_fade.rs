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
//! delay's end, which a frame drawn at once makes the same. Each scroll
//! container fades on its own, as each has its own controller in cc.

use std::collections::HashMap;
use std::time::Duration;

use vapors_dom::NodeId;
use vapors_elements::Document;
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

/// Each scroll container's fade, by container, and the fade of the floor
/// every container's thumbs share after the mouse.
#[derive(Clone, Debug, Default)]
pub(crate) struct ThumbFades {
    by_container: HashMap<NodeId, ThumbFade>,
    floor: Option<ThumbFade>,
}

impl ThumbFades {
    /// A finger touched at `now`, the mouse in use until then: the thumbs
    /// the mouse showed fade from now on the scroll timing, rather than
    /// vanishing in one frame (#405), and each container's own fade goes
    /// on untouched. A touch start reaches no scrollbar controller in cc:
    /// the controller's inputs are scrolls, the mouse and `DidRequestShow`
    /// (`scrollbar_animation_controller.cc:190-301`), called from
    /// `LayerTreeHostImpl::WillScrollContent`, `DidScrollContent` and the
    /// mouse handlers (`cc/trees/layer_tree_host_impl.cc:622-626`,
    /// `:4754-4796`, `:4970-5006`), so in Chrome a finger landing changes
    /// no thumb. The floor is the whole document's, so a scroll container
    /// added during this fade first draws its thumbs at the floor's
    /// opacity, as one added under the mouse draws them fully; in Chrome
    /// on Android a new scroller shows none until it scrolls. Kept so, as
    /// the mouse floor's behaviour; the event that calls this asks for the
    /// fade's first frame (`DocumentInput::handle`).
    pub(crate) fn touch_after_mouse(&mut self, now: Instant) {
        let mut floor = ThumbFade::default();
        floor.scrolled(now);
        self.floor = Some(floor);
    }

    /// The floor's opacity at `now`: what every container's thumbs are
    /// drawn at least at.
    pub(crate) fn floor_opacity(&self, now: Instant) -> f32 {
        self.floor.map_or(0.0, |floor| floor.opacity(now))
    }

    /// `container` scrolled at `now`.
    pub(crate) fn scrolled(&mut self, container: NodeId, now: Instant) {
        self.by_container
            .entry(container)
            .or_default()
            .scrolled(now);
    }

    /// `container`'s thumbs' opacity at `now`.
    #[cfg(test)]
    pub(crate) fn opacity(&self, container: NodeId, now: Instant) -> f32 {
        self.by_container
            .get(&container)
            .map_or(0.0, |fade| fade.opacity(now))
    }

    /// Each container's thumbs drawn at their opacity at `now`; those
    /// faded out are drawn hidden once and forgotten.
    pub(crate) fn apply(&mut self, document: &mut Document, now: Instant) {
        document.set_thumb_opacity(self.floor_opacity(now));
        for (container, fade) in &self.by_container {
            document.set_thumb_opacity_of(*container, fade.opacity(now));
        }
        self.by_container
            .retain(|_, fade| fade.phase(now) != Phase::Hidden);
    }

    /// Every faded container's own thumbs hidden and its fade dropped:
    /// the mouse's floor shows them all, and no fade needs a frame.
    pub(crate) fn hide_all(&mut self, document: &mut Document) {
        for container in self.by_container.keys() {
            document.set_thumb_opacity_of(*container, 0.0);
        }
        self.by_container.clear();
        self.floor = None;
    }

    /// How many containers are fading or shown.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.by_container.len()
    }

    /// Where the fades are at `now`: fading if any is, else shown until
    /// the earliest fade start, else hidden.
    pub(crate) fn phase(&self, now: Instant) -> Phase {
        let floor = self.floor.iter().map(|floor| floor.phase(now));
        let phases = self
            .by_container
            .values()
            .map(|fade| fade.phase(now))
            .chain(floor);
        let mut earliest = None;
        for phase in phases {
            match phase {
                Phase::Fading => return Phase::Fading,
                Phase::Shown(at) => earliest = Some(earliest.map_or(at, |e: Instant| e.min(at))),
                Phase::Hidden => {}
            }
        }
        earliest.map_or(Phase::Hidden, Phase::Shown)
    }
}

#[cfg(test)]
#[path = "thumb_fade_tests.rs"]
mod tests;
