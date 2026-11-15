# Touch momentum (fling) for scrolling: F2d's first slice

Tom, 2026-11-15, on Chrome on Android (Pixel 9 Pro): scrolling a vapors
list "stops dead" when the finger lifts. Chrome keeps it moving and slows
it down. This slice matches Chrome on Android (Q20). Out of scope: edge
stretch or glow, and desktop wheel momentum.

## Oracle

`scripts/fling-recorder/` records Chrome's own fling on a native overflow
list: every touch event (its `timeStamp` and `clientX`/`clientY`) and
every animation frame's `scrollTop`, on one clock (`performance.now()`).
A recording runs from a touchstart until the list has been still for
500 ms with the finger up. Each is saved as JSON beside the page server.
Tom records flings of several speeds on the Pixel, and the transcripts
become fixtures for the replay test (below).

## What Chrome does on Android, from its source

Read from Chromium `main` on 2026-11-15 (raw.githubusercontent.com,
chromium/chromium mirror). Line numbers are in those files on that day.

**Which curve.** `components/input/fling_controller.cc:459` makes the
curve with `WebGestureCurveImpl::CreateFromDefaultPlatformCurve`, passing
`scheduler_client_->ShouldUseMobileFlingCurve()`. On Android that is
`FlingSchedulerAndroid::ShouldUseMobileFlingCurve` →
`InputUtils::ShouldUseMobileFlingCurve()` (`components/input/utils.cc:62`),
true unless the `kDesktopFlingCurveOnAndroid` feature is on. With it
true, `ui/events/gestures/blink/web_gesture_curve_impl.cc:40-47` makes a
`MobileScroller` with the default `Config` (`chromecast_optimized`
false) and calls `Fling(0, 0, vx, vy, INT_MIN, INT_MAX, ...)`, so the
curve itself is unbounded. The content edge clamps the scroll, not the
curve. Pixels per inch (`input::kDefaultPixelsPerInch`) are passed but
not used by `MobileScroller`.

**The curve** (`ui/events/mobile_scroller.cc`, ported from Android's
`OverScroller` spline):

| constant | value | line | source |
|---|---|---|---|
| `kDefaultFriction` | 0.015 | 23 | `android.view.ViewConfiguration` scroll friction |
| `kDecelerationRate` | 2.3582018 | 26 | ln(0.78) / ln(0.9) |
| `kInflexion` | 0.35 | 29 | where the spline's tension lines cross |
| `kStartTension`, `kEndTension` | 0.5, 1.0 | 71-72 | spline tensions |
| `NUM_SAMPLES` | 100 | 134 | samples in the position table |
| `kThresholdForFlingEnd` | 0.1 px | 35 | ends the fling this close to the end |
| tuning coefficient | `ComputeDeceleration(0.84)` | 201 | g × 39.37 in/m × 160 px/in × 0.84 = 51 890.2 |

`ComputeDeceleration(f)` (lines 140-145) is
`kMeanGravityFloat (9.80665) × 39.37 × 160 × f`. The 160 is Android's
mdpi density, so velocities and distances are in DIPs.

For an initial speed `v` in DIP/s (lines 462-481):

- `l = ln(kInflexion × |v| / (friction × tuning))`
- duration = `exp(l / (kDecelerationRate − 1))` seconds
- distance = `friction × tuning × exp(kDecelerationRate / (kDecelerationRate − 1) × l)` DIPs

At time `t`, with `u = t / duration`, the position is
`start + spline_position(u) × distance`. `spline_position` is the
100-sample table built at lines 70-112 (a cubic Bézier with control
points from the tensions, inverted by bisection), interpolated linearly
between samples (lines 117-131). The velocity is the slope of that
segment × distance / duration. The fling ends when `u ≥ 1`, or within
0.1 px of the end (lines 405-447).

Worked values from those formulas:

| v (DIP/s) | duration | distance |
|---|---|---|
| 100 | 101.9 ms | 3.6 DIP |
| 500 | 333.3 ms | 58.3 DIP |
| 1000 | 555.2 ms | 194.3 DIP |
| 3000 | 1246.6 ms | 1308.9 DIP |
| 8000 | 2566.6 ms | 7186.4 DIP |

**The frame loop** (`components/input/fling_controller.cc`,
`ProgressFling`, lines 197-270, read at chromium 1555c8bf): until a
change has been sent, a frame two frames' time (1/30 s) or more after
the start moves the start to one frame (1/60 s) before that frame
(`kMaxMicrosecondsFromFlingTimestampToFirstProgress`), so a fling drawn
first late does not jump. Nothing moves at a frame at or before the
start, or before the last frame that sent a change. At each later frame the curve gives its offset
since the start (`WebGestureCurveImpl::Advance`), and the change since
the last frame is sent as a momentum scroll update, unless it is 0.1 px
or less on both axes (`kMinInertialScrollDelta`); the skipped change is
not carried to the next frame. Once the curve has ended, at its duration
or within 0.1 px of its end, the fling ends and that frame's change is
not sent. A momentum update the scroller does not consume, at the
content's edge, stops the fling
(`RenderWidgetHostViewInput::StopFlingingIfNecessary`,
`components/input/render_widget_host_view_input.cc:264-282`).
`crates/app/src/fling.rs` follows these rules. Each axis of a fling's
velocity is clamped to the maximum fling velocity, as the gesture
detector clamps the release velocity (`MAX_FLING_SPEED`, below under
the release velocity). A fling cannot start while a finger is down
(Chromium starts one only from a gesture's end, and a finger down stops
it). A wheel scroll or a button press stops a fling; vapors has no
keyboard scrolling yet, and when it does, a key scroll stops one too.
Losing the window's focus does not.

**The release velocity** (`ui/events/velocity_tracker/velocity_tracker.cc`):
the gesture detector's tracker defaults to `STRATEGY_DEFAULT = LSQ2`
(`velocity_tracker.h:95`), a second-degree least-squares fit over the
last 20 samples (`kHistorySize`, line 134) within 100 ms (`kHorizonMS`,
line 154). Samples older than 40 ms before a move, or 80 ms before the
up, count the pointer as stopped (`kAssumePointerMoveStoppedTimeMs`,
`kAssumePointerUpStoppedTimeMs`, lines 48 and 54).

**Thresholds** (`ui/events/gesture_detection/gesture_configuration_android.cc`):
the minimum and maximum fling velocities are Android's
`ViewConfiguration` values, read on the device
(`GetMinimumFlingVelocityInDipsPerSecond`,
`GetMaximumFlingVelocityInDipsPerSecond`). They are not in Chromium's
source. AOSP's defaults are 50 and 8000 DIP/s. The transcripts show the
Pixel's maximum is higher than 8000: 010 and 016 started at 13 214 and
15 502. vapors clamps at 16 000 (`MAX_FLING_SPEED`), that measured lower
bound rounded up, until #400 reads the Pixel's real value.

## Plan

1. **Oracle:** recordings from the Pixel, saved as fixtures (waiting on
   the page being served on the tailnet).
2. **Red:** `crates/app/tests/gpu_fling_replay.rs` starts each
   fixture's coast at Chrome's fitted start, offset and velocity on a
   page laid out like the recorder's, frames `ElementsApp` at Chrome's
   frame times, and asserts the offsets and the rest frame (see the
   Pixel findings below for why the drag is not replayed).
3. **Green:** the release velocity (LSQ2 over the last 100 ms), the
   curve above, the frame loop asking for frames while a fling runs
   (`Next::Wait` otherwise), a new touch stopping the fling, and the
   content edge stopping it.
4. **ORDER.md:** put momentum back on the F2d row in the same PR.

## The release velocity (PR 2)

`crates/platform/src/velocity_tracker.rs` ports LSQ2 as above, with the
history as Chromium keeps it (a ring of 20 that holds 19 samples after a
clear until it wraps), `least_squares.rs` ports `SolveLeastSquares`, and
the tests include Chromium's own `Basic`, `MaxVelocity`,
`VaryingVelocity` and `DelayedActionUp` on the same synthetic samples.
`Gestures` feeds the tracker the gesture's finger and, when that finger
lifts past the tap slop, sends `Event::Fling` at its velocity.
`DocumentInput` clamps each axis to the maximum, flings when either
axis is above the minimum (`GestureDetector`'s `Action::UP` path), and
flings the container the gesture scrolled.

- **Minimum 50 CSS px/s:** AOSP's `ViewConfiguration` default; nothing in
  the recordings contradicts it.
- **Maximum 16 000 CSS px/s:** AOSP's default is 8000 dp/s, but 010 and
  016 started at 13 214 and 15 502, so the Pixel's maximum is higher and
  unknown. 16 000 is that lower bound rounded up; a recording of the
  fastest flick Tom can make would bound it from the other side.
- **RELAXED, two fingers:** a gesture that had a second finger does not
  fling. Chromium's `GestureDetector` flings at `UP` at the last
  finger's own velocity (its history kept through the second finger's
  down), unless a finger lifted moving against another, which clears
  the tracker (`POINTER_UP`'s dot product). Retired when a two-finger
  fixture is recorded.
- **RELAXED, times:** touches are stamped when winit hands them over;
  winit 0.30 gives touches no time on any platform. (On the web, real
  times would take a pointer listener of our own reading `timeStamp`
  and matching winit's coalesced touches by id and position; natively
  there is nothing to read.) The tracker's 40 ms and 80 ms stop rules
  then also fire when the page did not handle input for that long, and
  moves handled together share a time. The tracker answers in three
  states: moving at a velocity, stopped, or unresolvable, when a 40 ms
  gap is followed by moves handled together (a stall) or the samples'
  times leave the fit no solution. An unresolvable lift does not fling;
  it is counted (`Gestures::unmeasured_lifts`) and named in a debug
  build. An up handled 80 ms late after a stall alone cannot be told
  from a finger held still, and answers stopped. The fling starts at
  the lift's handling time, on the clock frames are drawn on, where
  Chromium starts it at the last scroll update's. Retired by #403 (real
  touch times; upstream rust-windowing/winit#1194).
- **Not ported:** scroll rails (`SnapScrollController`), which zero the
  off-axis velocity of a mostly straight fling; the recorded flings are
  vertical lists, where it changes nothing.

## Thumbs on touch (Tom, 2026-11-15)

Tom on the first page: the thumb vanished at once when a scroll ended,
and at once when a scroll met the top or bottom. The cause: on touch the
thumbs were an on/off flag, "a gesture is scrolling or a fling runs";
a fling meeting the edge ends that frame, so the flag went off and the
thumb vanished, as at a fling's natural end or a lift with no fling.

`crates/app/src/thumb_fade.rs` now draws them as Chrome on Android does
(cc's `ANDROID_OVERLAY` animator, `cc/input/
scrollbar_animation_controller.cc`): each scroll update, the drag's or
the fling's, shows them at full opacity at once (there is no fade in),
and 300 ms after the last update they fade out linearly over 300 ms
(`scrollbar_fade_delay` and `scrollbar_fade_duration`,
`InitializeScrollbarFadeAndDelay` in `third_party/blink/renderer/
platform/widget/compositing/layer_tree_settings.cc`). A fling stopped by
the edge last scrolled on the frame that met it, so the fade follows
300 ms later, as everywhere else. The app waits for the fade's start
(`Next::WaitUntil`), draws each fade frame, then asks for none. The
document draws thumbs at an alpha (`Document::set_thumb_opacity`, 256
steps). Every scroll container's thumbs share the one fade, as they
shared the flag; Chrome fades each container's own.

## What the Pixel recorded (2026-11-15)

27 recordings by Tom on the Pixel 9 Pro, Chrome 157, Android, device
pixel ratio 2.75 (`~/.cache/fling-recordings`, made with the recorder
above). `scripts/fling-recorder/fit.py coast` fits the curve above to
each coast's first 12 frames alone: the start frame, its time to the
half millisecond, and the starting velocity. Nothing later in the coast
is seen by the fit.

**The curve is MobileScroller's.** Three coasts ran to rest away from
the list's ends (006, 021, 023); fitted on their first 12 frames, the
curve's duration and distance match where they came to rest:

| recording | starting velocity | curve | recorded coast | worst frame after the fit's 12 |
|---|---|---|---|---|
| 006 | 3134 DIP/s | 1287 ms over 1412 px | 1267 ms over 1424 px | 11.7 px |
| 021 | 3140 DIP/s | 1289 ms over 1417 px | 1267 ms over 1410 px | 13.6 px |
| 023 | 2938 DIP/s | 1228 ms over 1262 px | 1200 ms over 1268 px | 8.7 px |

003, 005 and 022 do not show a duration: the next touch cut each
while it still moved 1 to 3 px a frame (`"ended": "interrupted"`), so
their recorded coasts are the time until that touch. (An earlier
version of this table listed 003 and 005 as coasts to rest.) Over the
first 12 coast frames every fling fits within 2.1 to 4.6 px.

**The content edge stops a fling.** 010 and 016 run to the list's
ends (18 901.5 and 0) and 019 to its top: their coasts end there, cut
short of the curve.

**Boost is there.** A fling started in the same direction while one is
still running starts faster than its own drag explains: 008 started at
4125 DIP/s, then 009, flung during it, at 8825; 013, 014 and 015 at
2525, 7450 and 10 950. That is Chrome's `FlingBooster`
(`ui/events/blink/fling_booster.cc`): the new fling's velocity plus the
previous fling's starting velocity, when both are at least 350 DIP/s
(`kMinBoostFlingSpeedSquare`), they point the same way, and the new
fling comes within its cutoff of the last scroll. A new touch with no
fling (a tap, or a drag that stops) cancels the running fling.

**Taps:** 001, 011, 012, 017 and 020 have no coast.

**Why the touch replay is not the oracle.** Chrome delivers only the
first few touchmoves to a passive listener once it scrolls (five touch
events per fling here: a touchstart, three moves, the touchend), so the
recordings do not hold the samples Chrome's velocity tracker used. The
drag does show in `scrollTop`, but `scrollTop` lags the finger: a
release velocity rebuilt from the drag's frames comes out at about half
the starting velocity Chrome's coast shows. A replay of the drag would
hold vapors to that lag, not to Chrome's tracker.

So the replay starts each coast where Chrome's started: at the start
time and velocity fitted to its first 12 frames, from that frame's
offset (`scripts/fling-recorder/fit.py coast`, written beside each
fixture as a `.coast.json`). The rest of the coast, the rest frame and
the edge are checks the fit never saw. It holds the
curve, the frame loop and the edge stop to the recorded frames: within
6 px over the first 12 coast frames (the fit came within 2.1 to 4.6 px
of them), within 1.2% of the distance the coast travelled after them,
coming to rest within one frame of Chrome on the six that came to rest,
at the list's end where Chrome's coast stopped at it, and asking for no
frame within two frames of its last movement. The 1.2% is measured out
of sample, the worst whole-coast error over distance travelled, rounded
up:

| fixture | worst frame | travelled | ratio |
|---|---|---|---|
| 003 (cut) | 16.75 px | 1485 px | 0.0113 |
| 005 (cut) | 10.67 px | 1196 px | 0.0089 |
| 006 | 11.70 px | 1424 px | 0.0082 |
| 010 (edge) | 20.63 px | 3479 px | 0.0059 |
| 016 (edge) | 34.06 px | 7061 px | 0.0048 |
| 019 (edge) | 5.44 px | 650 px | 0.0084 |
| 021 | 13.61 px | 1410 px | 0.0097 |
| 022 (cut) | 12.60 px | 1287 px | 0.0098 |
| 023 | 8.69 px | 1268 px | 0.0068 |

It still catches a real error: friction or the deceleration constant
10% off either way, or the start a frame (16.7 ms) early or late, each
fails the replay. The one frame is Chrome's last moving frame, which
moves 0.4 to 0.7 px past where the fitted curve ends (006, 021, 023);
#395 asks whether Chrome's fling resampling explains it. The release velocity is a port of
Chromium's LSQ2 tracker (`ui/events/velocity_tracker/`), tested against
that tracker's own behaviour on synthetic samples.

**Fixtures for the replay:** coasts to rest, 006, 021 and 023;
coasts cut by the next touch, 003, 005 and 022 (held to their path, with
no rest frame); and the edges, 010 and 016 (the list's two ends) and
019 (its top). The touches that cut or followed them came below
350 px/s, so no boost. All nine are held to the same allowances. Boost, 008 to 009 and 013 to 015, goes with the slice that
builds it.

**First PR or later:** the first PR builds the curve, the frame loop, a
new touch stopping the fling and the edge stopping it, started by
`ElementsApp::fling` and held to the coast and edge fixtures. The
release velocity (the LSQ2 port, fed touch times by the platform's
`Gestures`, which starts the fling at a touch's end) is the second.
Boost is the third, held to the boost chains.
