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
2. **Red:** `crates/app/tests/gpu_fling_replay.rs` replays each
   transcript's touches through `Gestures` and `ElementsApp` on a page
   laid out like the recorder's. It asserts the scroll offset at each of
   Chrome's frames within a stated tolerance, and pins the end position
   and the time the list stops.
3. **Green:** the release velocity (LSQ2 over the last 100 ms), the
   curve above, the frame loop asking for frames while a fling runs
   (`Next::Wait` otherwise), a new touch stopping the fling, and the
   content edge stopping it.
4. **ORDER.md:** put momentum back on the F2d row in the same PR.
