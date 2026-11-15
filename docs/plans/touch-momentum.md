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
`ProgressFling`, read at chromium 1555c8bf): nothing moves at or before
the fling's start time. At each later frame the curve gives its offset
since the start (`WebGestureCurveImpl::Advance`), and the change since
the last frame is sent as a momentum scroll update, unless it is 0.1 px
or less on both axes (`kMinInertialScrollDelta`); the skipped change is
not carried to the next frame. Once the curve has ended, at its duration
or within 0.1 px of its end, the fling ends and that frame's change is
not sent. A momentum update the scroller does not consume, at the
content's edge, stops the fling
(`RenderWidgetHostViewInput::StopFlingingIfNecessary`,
`components/input/render_widget_host_view_input.cc:264-282`).
`crates/app/src/fling.rs` follows these rules.

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
source. AOSP's defaults are 50 and 8000 DIP/s, but the Pixel's are
unverified until the transcripts show them.

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

## What the Pixel recorded (2026-11-15)

27 recordings by Tom on the Pixel 9 Pro, Chrome 157, Android, device
pixel ratio 2.75 (`~/.cache/fling-recordings`, made with the recorder
above). `scripts/fling-recorder/fit.py` fits the curve above to each
coast: the start frame, its time to the half millisecond, and the
starting velocity, chosen so the curve ends at the recorded resting
offset (a fling that ran to rest) or best fits the first 12 coast
frames (one cut short by the next touch or the list's end).

**The curve is MobileScroller's.** Every fling that ran to rest away
from the list's ends fits it:

| recording | starting velocity | curve | recorded coast | worst frame |
|---|---|---|---|---|
| 023 | 3015 DIP/s | 1251 ms over 1320 px | 1233 ms | 7.3 px |
| 003 | 3226 DIP/s | 1315 ms over 1485 px | 1250 ms | 8.5 px |
| 021 | 3131 DIP/s | 1286 ms over 1410 px | 1266 ms | 7.8 px |
| 005 | 2848 DIP/s | 1200 ms over 1196 px | 1066 ms | 6.3 px |

Over the first 12 coast frames every fling fits within 3 to 5 px. The
worst frames over a whole coast are where the recorded offset is
quantised to device pixels (1/2.75 CSS px) and the fit's start time is
a frame's.

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

So the replay starts each coast where Chrome's started: at the fitted
start time, offset and velocity (`scripts/fling-recorder/fit.py
--joint`, written beside each fixture as a sidecar JSON). It holds the
curve, the frame loop and the edge stop to the recorded frames: within
6 px over the first 12 coast frames and 11 px over the whole coast,
coming to rest within one frame of Chrome, and at the list's end where
Chrome's coast stopped at it. The allowances are recording noise, not
slack for vapors: they are the worst error of Chrome's own curve fitted
to Chrome's recordings (5.6 px and 10.1 px, on 010), rounded up. The
one frame is Chrome's last moving frame, which moves 0.4 to 0.7 px past
where its fitted curve ends (006, 021, 023); #395 asks whether Chrome's
fling resampling explains it. The release velocity is a port of
Chromium's LSQ2 tracker (`ui/events/velocity_tracker/`), tested against
that tracker's own behaviour on synthetic samples.

**Fixtures for the replay:** coasts to rest, 023, 003, 005, 006, 021
and 022 (the last five each cut by the next touch below 350 px/s, so no
boost and almost no distance lost), and the edges, 010 and 016 (the
list's two ends) and 019 (its top). All nine are held to the same
allowances. Boost, 008 to 009 and 013 to 015, goes with the slice that
builds it.

**First PR or later:** the first PR builds the curve, the frame loop, a
new touch stopping the fling and the edge stopping it, started by
`ElementsApp::fling` and held to the coast and edge fixtures. The
release velocity (the LSQ2 port, fed touch times by the platform's
`Gestures`, which starts the fling at a touch's end) is the second.
Boost is the third, held to the boost chains.
