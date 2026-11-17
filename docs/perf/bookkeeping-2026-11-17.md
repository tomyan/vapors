# B3's bookkeeping, once per sheet and once per key (2026-11-17)

The attribution in `first-frame-2026-11-16.md` put #437's +30.0 M
instructions (judged, Mount page, 10,000 rows) in two pieces of
bookkeeping:
- 11.1 M: hashing a sheet's text on every mount.
- 16.7 M: looking up the key's compiled rules on every keyed element.

This change makes both run once:
- The hash is checked when a sheet, its text or its claimed hash is
  new (`style/compiled_add.rs`).
- Check 1 runs when a key's sharing answer is decided, once per key
  per `Kept` (`restyle_share.rs`).
- `compiled_key` looks the key up once on a hit.

The release verification is kept.

Measured with the attribution's command and profile (`vapors-perf
--first-frame 10000`, judged, median of 3 runs; outputs in
`bookkeeping-2026-11-17/`), but not its settled load: these runs started
at load averages of 7.9 to 18.0, where the attribution settled to 1.43.
Only the instruction counts are quoted, since they move little with
load (the spread between runs is under 0.05 M); the times in the raw
files are not comparable and are not quoted.

"This change" was measured at e4be3ecd, a commit the squash merge
removes. The merged tree differs from it in: an `if let` chain
(formatting); `refuse_key` also dropping the key's sharing decision,
which runs only when a key is refused; and a count of sheet hashes,
which rises only when a hash is taken. None of these runs per element
or per mount on this page.

| point | revision | mount | start: style |
|---|---|---|---|
| before B3 | b2cabe32 | 307.53 M | 429.80 M |
| main | 132ba1da | 318.84 M | 448.26 M |
| this change | e4be3ecd | 307.97 M | 431.91 M |

Against main: mount −10.87 M, style −16.35 M. Against before B3,
#437's cost goes from +30.0 M to +2.55 M: mount +0.44 M, style
+2.11 M. Of the attribution's parts, release verification (1.3 M) and
the compiled computation (0.5 M) remain. Compiled winners still buy no
first-frame time; layout is the first frame's cost (606 of 909 ms at
10,000 rows warm, `first-frame-2026-11-16.md`).
