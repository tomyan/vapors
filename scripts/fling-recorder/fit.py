"""Fits Chromium's Android fling curve (MobileScroller) to the coasts the
fling recorder saved.

  python3 fit.py summary <fling.json>...
      per recording: the starting velocity, the curve's length and the
      worst coast frame of a fit to the first 12 coast frames

  python3 fit.py coast <out-dir> <fling.json>...
      per recording: the coast's start time, offset and velocity fitted
      jointly to its first 12 frames (6 px allowance) and the whole coast
      (11 px), from the coast frame at `first_frame_ms` on, among those
      whose replay (as crates/app/src/fling.rs moves it) comes to rest
      within a frame of Chrome's, written as
      <out-dir>/<name>.coast.json for the replay test
      (crates/app/tests/gpu_fling_replay.rs)

The constants and the spline are ui/events/mobile_scroller.cc's, read at
chromium 1555c8bf68dca9c1c73be452fa5a982afe33fdda (see
docs/plans/touch-momentum.md).
"""
import json, math, os, sys

INF, ST, ET, N = 0.35, 0.5, 1.0, 100
P1, P2 = ST * INF, 1 - ET * (1 - INF)
G = 9.80665
TUNE = G * 39.37 * 160 * 0.84
FR = 0.015
DR = 2.3582018
FIRST_FRAMES, FIRST_ALLOWANCE, WHOLE_ALLOWANCE, REST_FRAMES_APART = 12, 6.0, 11.0, 1


def table():
    pos = [0.0] * (N + 1)
    xmin = 0.0
    for i in range(N):
        a = i / N
        xmax = 1.0
        while True:
            x = xmin + (xmax - xmin) / 2
            c = 3 * x * (1 - x)
            tx = c * ((1 - x) * P1 + x * P2) + x**3
            if abs(tx - a) < 1e-5:
                break
            if tx > a:
                xmax = x
            else:
                xmin = x
        pos[i] = c * ((1 - x) * ST + x) + x**3
    pos[N] = 1.0
    return pos


POS = table()


def spline(u):
    if u >= 1:
        return 1.0
    i = int(N * u)
    return POS[i] + (u - i / N) * (POS[i + 1] - POS[i]) * N


def log_term(v):
    return math.log(INF * abs(v) / (FR * TUNE))


def dur(v):
    return math.exp(log_term(v) / (DR - 1))


def dist(v):
    return FR * TUNE * math.exp(DR / (DR - 1) * log_term(v))


def load(path):
    """The recording's coast frames (after the last touch end), its
    direction, and the list's scroll range."""
    d = json.load(open(path))
    ev = d["events"]
    ends = [e["t"] for e in ev if e["kind"] in ("touchend", "touchcancel")]
    if not ends:
        sys.exit(f"{path}: no touchend; record a fling, not a scroll")
    frames = [(e["t"], e["top"]) for e in ev if e["kind"] == "frame" and e["t"] > ends[-1]]
    if len(frames) < FIRST_FRAMES:
        sys.exit(f"{path}: {len(frames)} coast frames, fewer than {FIRST_FRAMES}; a tap?")
    sign = 1 if frames[min(5, len(frames) - 1)][1] > frames[0][1] else -1
    lst = d["list"]
    return frames, sign, (0.0, lst["scrollHeight"] - lst["height"])


def position(t, start_ms, start_top, velocity, scroll_range):
    lo, hi = scroll_range
    sign = 1 if velocity > 0 else -1
    u = max(0.0, (t - start_ms) / 1000 / dur(velocity))
    return min(hi, max(lo, start_top + sign * dist(velocity) * spline(u)))


def errors(frames, k, coast, scroll_range):
    return [abs(position(t, *coast, scroll_range) - top) for t, top in frames[k:]]


def first_frames_fit(frames, sign):
    """Coarse then fine search over the start frame, velocity and start
    time against the first coast frames alone."""
    def err_of(k, v, dt0):
        t0, p0 = frames[k]
        return max(abs(position(t, t0 + dt0, p0, sign * v, (-1e9, 1e9)) - top)
                   for t, top in frames[k:k + FIRST_FRAMES])
    _, k, v0, d0 = min((err_of(k, v, dt0), k, v, dt0)
                       for k in range(1, 4) for v in range(300, 20000, 50) for dt0 in range(-20, 21, 2))
    _, k, v, dt0 = min((err_of(k, v, h / 2), k, v, h / 2)
                       for v in range(v0 - 50, v0 + 51, 2) for h in range(int(2 * d0) - 4, int(2 * d0) + 5))
    return k, v, dt0


MIN_DELTA, END_THRESHOLD, REST = 0.1, 0.1, 0.5


def replayed(frames, coast, scroll_range):
    """The offset at each frame as vapors' fling moves it (crates/app/src/
    fling.rs, after Chromium's FlingController): no delta of 0.1 px or
    less, none once the curve ends (at its duration or within 0.1 px of
    its end), and the fling stopped by a delta that moves nothing."""
    start_ms, top, velocity = coast
    lo, hi = scroll_range
    T, D, sign = dur(velocity), dist(velocity), (1 if velocity > 0 else -1)
    reached, running, out = 0.0, True, []
    for t, _ in frames:
        u = (t - start_ms) / 1000 / T
        if running and u > 0:
            covered = spline(u) * D if u < 1 else D
            if u >= 1 or D - covered < END_THRESHOLD:
                running = False
            else:
                delta, reached = covered - reached, covered
                moved = min(hi, max(lo, top + sign * delta)) if abs(delta) > MIN_DELTA else top
                running = moved != top or abs(delta) <= MIN_DELTA
                top = moved
        out.append(top)
    return out


def last_moving(offsets):
    return max((i for i, o in enumerate(offsets) if abs(o - offsets[-1]) > REST), default=None)


def joint_fit(path):
    frames, sign, scroll_range = load(path)
    k, v_first, dt_first = first_frames_fit(frames, sign)
    t0, p0 = frames[k]
    best = None
    for v in range(v_first - 200, v_first + 201, 10):
        for dt0 in range(-8, 9):
            for dp in [x / 2 for x in range(-8, 9)]:
                coast = (t0 + dt_first + dt0, p0 + dp, sign * v)
                ours = replayed(frames[k:], coast, scroll_range)
                rest = (last_moving(ours), last_moving([top for _, top in frames[k:]]))
                if None in rest or abs(rest[0] - rest[1]) > REST_FRAMES_APART:
                    continue
                e = [abs(o - top) for o, (_, top) in zip(ours, frames[k:])]
                score = max(max(e[:FIRST_FRAMES]) / FIRST_ALLOWANCE, max(e) / WHOLE_ALLOWANCE)
                if best is None or score < best[0]:
                    best = (score, coast, max(e[:FIRST_FRAMES]), max(e))
    if best is None:
        sys.exit(f"{path}: no start, velocity and time near the first frames' fit comes to rest within a frame of Chrome's")
    _, (start_ms, start_top, velocity), first, whole = best
    return {"first_frame_ms": t0, "start_ms": round(start_ms, 2), "start_top": round(start_top, 3),
            "velocity": velocity, "first12_px": round(first, 2), "whole_px": round(whole, 2)}


def summary(path):
    frames, sign, scroll_range = load(path)
    k, v, dt0 = first_frames_fit(frames, sign)
    t0, p0 = frames[k]
    worst = max(errors(frames, k, (t0 + dt0, p0, sign * v), scroll_range))
    name = os.path.basename(path)
    print(f"{name}: v {sign * v} dip/s, curve {dur(v) * 1000:.0f} ms over {dist(v):.1f} px; "
          f"worst coast frame {worst:.2f} px")


def coast(out_dir, path):
    fitted = joint_fit(path)
    name = os.path.basename(path).removesuffix(".json")
    with open(os.path.join(out_dir, f"{name}.coast.json"), "w") as f:
        json.dump(fitted, f, indent=1)
        f.write("\n")
    print(name, fitted)


def main(argv):
    if len(argv) >= 2 and argv[0] == "summary":
        for p in argv[1:]:
            summary(p)
    elif len(argv) >= 3 and argv[0] == "coast":
        for p in argv[2:]:
            coast(argv[1], p)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
