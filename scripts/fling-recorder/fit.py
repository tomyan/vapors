"""Fits Chromium's Android fling curve (MobileScroller) to the coasts the
fling recorder saved.

  python3 fit.py summary <fling.json>...
      per recording: the starting velocity, the curve's length and the
      worst coast frame of a fit to the first 12 coast frames

  python3 fit.py coast <out-dir> <fling.json>...
      per recording: the coast's start time and velocity fitted to its
      first 12 frames alone (from `first_frame_ms`, whose offset is the
      start), written as <out-dir>/<name>.coast.json for the replay test
      (crates/app/tests/gpu_fling_replay.rs). The rest of the coast, its
      rest frame included, is left for the test to check

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
FIRST_FRAMES = 12


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


def early_fit(path):
    """The coast's start time and velocity fitted to its first
    FIRST_FRAMES frames alone, from the frame it starts on: nothing later
    in the coast, its rest frame included, is seen by the fit."""
    frames, sign, _ = load(path)
    k, v, dt0 = first_frames_fit(frames, sign)
    t0, p0 = frames[k]
    return {"first_frame_ms": t0, "start_ms": round(t0 + dt0, 2), "start_top": p0,
            "velocity": sign * v, "fit_frames": FIRST_FRAMES}


def summary(path):
    frames, sign, scroll_range = load(path)
    k, v, dt0 = first_frames_fit(frames, sign)
    t0, p0 = frames[k]
    worst = max(errors(frames, k, (t0 + dt0, p0, sign * v), scroll_range))
    name = os.path.basename(path)
    print(f"{name}: v {sign * v} dip/s, curve {dur(v) * 1000:.0f} ms over {dist(v):.1f} px; "
          f"worst coast frame {worst:.2f} px")


def coast(out_dir, path):
    fitted = early_fit(path)
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
