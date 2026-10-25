# 11. Testing and oracles

Status: design, 2026-10-25. Each area document names its oracles; this
page says how they are run, what counts as an oracle, and what the
tiers are. Built already: the Chromium layout oracle, the paint
comparison with three rules, the text ink band, the `.pixels` checks
shared by native and web, the recording `Dom`, the seeded differential
test, the page-contract script, nextest tiers with deadlines.

## What an oracle is

A source of expected behaviour that vapors did not write. In order of
preference:

1. **A browser.** Headless Chromium laying out, painting, dispatching
   or exposing the same HTML and CSS. Used wherever the semantics are
   the web's: layout, paint, cascade, events, focus, a11y roles and
   names, transitions at a time.
2. **A specification's test suite.** web-platform-tests, ported case by
   case where headless Chromium cannot run them for us or where the
   assertion is on a computed value we can read directly. Svelte 5's
   runtime samples for reactive ordering and `await`.
3. **A reference implementation's tests.** `reactive_graph` for signals
   (done); Taffy's generated fixtures for layout.
4. **A reference render.** A committed image rendered twice for
   determinism, for what no browser renders (a wgpu scene, the glyph
   atlas's exact pixels).
5. **A recorded transcript.** For behaviour with no external
   definition and no browser equivalent (momentum curve, an IME
   composition, a pen stroke), recorded once from the real device and
   replayed.
6. **A budget.** A measured number with a stated ceiling, for
   performance.
7. **None**, with the reason stated in the area document (a boundary
   component, which the HTML has no element for).

A golden image vapors itself produced is not an oracle; it is a
regression pin, and the area documents call it that.

## Running web-platform-tests

WPT has two kinds of test. **Reftests** are two pages that must render
alike (`<link rel="match">`, pixel-exact at 800x600 unless a
`<meta name=fuzzy>` states a tolerance; `class="reftest-wait"` defers
the screenshot until the page removes it); the oracle runner loads both
in Chromium and vapors renders the test page, and the three paint rules
compare vapors' render with Chromium's render of the *reference*, with
the test's fuzz added to the rules' tolerance. Reftests need no wpt
server, which is why they are the first kind taken. A reftest
whose reference uses features vapors refuses is skipped by name with
the reason in a committed list, so the skipped set is visible and
shrinks. **testharness.js** tests assert in JavaScript; the useful ones
read `getComputedStyle`, `getBoundingClientRect`, event logs or
`document.activeElement`. These are ported by hand to Rust tests that
assert the same values, with the WPT path in the test name, and the
original is run in Chromium by the oracle runner to confirm the
expected value. testharness tests need a wpt checkout and a server
whose root is that checkout (they load `/resources/testharness.js` and
their own helpers by absolute path), so the runner serves a pinned
checkout; `pointerevents/` and `uievents/` tests drive input through
testdriver, which needs the wpt runner, so the runner performs those
actions itself through DevTools (`Input.dispatchMouseEvent` and friends)
in place of testdriver. Directories in use per area: `css/css-flexbox`,
`css/css-grid`, `css/css-cascade`, `css/selectors`, `css/css-variables`
(4); `css/css-transitions`, `css/css-animations` (6);
`css/css-transforms` (9); `pointerevents`, `uievents`,
`html/semantics/forms`, `html/interaction/focus` (3); `accname`,
`html-aam`, `wai-aria` (5). WPT is BSD-3 licensed; fixtures copied in
keep their header.

## The oracle runner

One Rust crate (`vapors-elements-corpus` today, grown rather than
duplicated) drives headless Chromium with the rules in CLAUDE.md
(deadline, stderr drained, `DBUS_SESSION_BUS_ADDRESS` stripped) and
adds the DevTools protocol over a pipe for what `--dump-dom` cannot
give: `Input.dispatchMouseEvent`/`dispatchKeyEvent` for event logs and
focus, `Accessibility.getFullAXTree` for roles and names,
`Animation.seekAnimations` (or virtual time) for transitions,
`Emulation.setDeviceMetricsOverride` for scale. Outputs are committed
JSON beside the fixtures and checked in CI (`--check`), as the layout
oracle does, so a Chromium that changes its answer is a visible diff.

## Tiers

| Tier | Runs | Needs | Deadline |
|---|---|---|---|
| unit | Every crate's tests without a device or a browser: signals, compiler, CSS, elements with `RecordingDom` and `Document`, the recorder's format, remote shape refusals | Nothing | nextest per-test |
| gpu | Offscreen render on the selected adapter: paint comparisons, text ink, demos' pixels, scenes' reference renders, determinism (twice), budgets | A Vulkan adapter (llvmpipe in CI) | per test |
| web | The built page in headless Chromium: page contract, pixels per demo, idle frames, offload round trip, the a11y mirror's tree, worker mode | Chromium, trunk | per case |
| oracle | Regenerate and check every committed Chromium answer | Chromium | per fixture |
| scenario | Recorded or scripted sessions replayed against the build under Xvfb and in Chromium, judged by the UX review (area 10) | Both | per scenario |
| screenshot | Native under Xvfb, held to `.pixels` | Xvfb | per target |

A tier that cannot run fails naming what is missing (rule). Tests run
in random order with no retries; a flake is a bug with a deadline.

## Test doubles that are part of the design

- `RecordingDom`: every `Dom` call recorded, so a test proves a signal
  change patched one node. Area 1's held-update tests assert the order
  and count of calls after futures are resolved by hand.
- A stepped clock: `platform::Instant` is injectable; no test sleeps.
- A manual executor: futures are polled when the test says, so a
  pending version's hold is observable.
- A fake AccessKit adapter: records `TreeUpdate`s and sends
  `ActionRequest`s.
- A fake platform (`Platform::detached`, built) and fake services with
  scripted answers including `Unreachable` on the Nth call.
- A stand-in server on a second origin for remote functions.

## Proving a check

Every new check is broken once, watched to fail, and restored, and the
commit says so (rule). For an oracle comparison this means perturbing
the fixture or the engine and seeing the comparison fail; S3 did this
for the paint rules by perturbing Chromium's own rendering, and every
new rule follows that pattern. A counted test (elements restyled, draws
per frame, updates per frame) states how it was broken.

## Budgets

Measured on this machine and recorded in the area document with the
date and the load; the ceiling is set from the measurement with a
stated margin, and the test fails over it in the `gpu` tier. Phone
numbers come from recordings (area 10) and are recorded beside them,
never asserted in CI.

## Taken from where

- Chromium as the layout oracle: Taffy's own fixtures are generated the
  same way, and Blitz compares against it.
- WPT: the web's shared test suite; Servo and Ladybird run it as their
  conformance measure.
- Svelte's `runtime-runes` samples: the shape (`html` before and after,
  a `test` that drives and asserts) ported to `RecordingDom`.
- sumi's `sumitest` for deterministic scenario rendering with no
  animation stepping.
- The factory's own rules (FACTORY.md 2.8): property tests, mutation
  testing on the diff, fuzzing of every parser, flaky means broken.

## Deliberately left out

- Visual review by a model of every screenshot: the UX review judges
  recorded scenarios, not pixel diffs; pixel rules are deterministic.
- Running the WPT harness itself (`wpt run`): it needs its runner and
  a browser driver; the oracle runner takes what it needs from the
  tests.
- Firefox and Safari as oracles: one browser is one answer; where they
  differ from Chromium on a specification point, the specification is
  consulted and the choice recorded.

## Slices

| Slice | Delivers | Pulled by |
|---|---|---|
| Q1 DevTools in the oracle runner | Events, focus, a11y tree, virtual time | Area 3's E1 and area 5's A1 |
| Q2 WPT reftests as paint comparisons | The skip list, the runner, the first directories | Area 9's T1 and area 6's A6 |
| Q3 stepped clock and manual executor | | Area 1's R2 and area 6's A1 |
| Q4 scenario tier | Replay against a build, judged | Area 10's C4 |
| Q5 budgets in CI | Per-phase ceilings | Area 2's S1 |
