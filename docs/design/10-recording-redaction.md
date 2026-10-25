# 10. Session recording and redaction

Status: design, 2026-10-25. Nothing is built. Tom's direction
(2026-10-24): vapors records sessions as element-tree snapshots plus
input events plus timings, not video; user content (the photo, typed
text) is tagged and replaced at the source; a privacy test proves no
image bytes reach a recording; dev builds record by default with an
indicator and a toggle, users opt in with a retention limit; uploads go
through `/events` (area 8); an xtask replays sessions into frames at
device size; a UX review agent reads the frames; scripted scenarios are
judged per pull request.

## The model

**A recording is a log of what the frame loop already knows.** Area 2's
frame has phases; the recorder is phase 7, and it writes:

| Record | Content | Source |
|---|---|---|
| `Meta` | App name, build stamp, target (native, web, worker), viewport size and scale, `prefers-*`, the redaction policy in force, the IME path | Start-up and on change |
| `FullSnapshot` | The element tree: tags, attributes, text (redacted), computed style ids, boxes; the stylesheets by name and hash (not text, for scoped component styles; text for global sheets, which are the app's) | First frame, and every N seconds or on request, so replay can start anywhere |
| `Mutation` | The `Dom` calls since the last frame (create, set attribute, set text, insert, remove), as the tree's own change list; plus boxes that moved and style ids that changed | The incremental update's dirty marks |
| `Input` | Every platform event as received, with its timestamp, in physical pixels; key events with `key` redacted to a class in text fields (below) | Phase 1 |
| `Dispatch` | Each dispatched event's target and whether it was prevented; focus changes; capture | Area 3 |
| `Async` | A pending version opened and settled, with its duration; a future dropped; a remote call's outcome by name (not body) | Areas 1 and 8 |
| `Timing` | Per-phase durations, frame interval, frames over budget | Area 2's spans |
| `Report` | Every refusal, error and status line | `report_*` |
| `App` | Events the app marks for recording with a redaction class (touché's P0 events: a turn's tokens, a correction) | The app |

Records are length-prefixed CBOR; one file per session with a header;
the same format on both targets; written through area 7's storage to a
per-app recordings directory (OPFS on the web) and uploaded in chunks by
a `command` when the user has opted in and the connection allows.

**Replay is the recorder's inverse.** `cargo xtask replay <session>
--at WxH@scale --frames out/` rebuilds the `Document` from the snapshot
and mutations and paints each frame offscreen with the real painter, so
the frames are what the person saw, less the redacted content. The
recorded input is drawn over the frames as a cursor and touch marks; it
is not dispatched, because the mutations already hold every effect it
had, and dispatching it again would run default actions (a scroll, a
toggle) twice. Only a scenario (C4) dispatches input, against a built
app. Replay never runs app code: it replays the tree, not the signals,
so it needs no model key, no engine and no network. A recorded style sheet hash
whose text is not in the build is a refusal naming the hash.

## Redaction at the source

The rule: content a person put into the app never enters a record.
What enters is a description that preserves layout.

| Content | Tag | What is recorded |
|---|---|---|
| The photo and every texture derived from it (thumbnails, histograms of it) | `img`, `canvas` and the engine's textures carry `data-user-content="image"` set by the element, not by the app | Size, aspect, a four-colour tone tile (mean of each quadrant) so replay shows a plausible block; never pixels. Histograms and readouts of the image are numbers about it and are recorded (Tom's events need them), tagged `measurement` |
| Text the person typed (composer, key fields) | `input`, `textarea` carry it by default; `data-user-content="text"` on any element whose text comes from the person | Length in graphemes and the script; replay shows `•` runs of the same width class. Key events in a text field record only the class (letter, digit, punctuation, control) |
| Text the agent wrote | `data-user-content="generated"` on the message element | Recorded, since it is the product's output; a per-app policy can mask it |
| UI strings (labels, menus, patterns) | None | Recorded |
| File names | `data-user-content="name"` | Extension and length |
| Keys and secrets | Never in the tree: the key sheet's field is `type=password`, recorded as `text` with length zero | |

The element tree carries the tags, so the recorder does not know touché:
a component author tags user content once, in the template, and every
record that would carry its text or pixels consults the tag. An `img`
without a source tag is refused in debug: every image says whether it
is user content, so a forgotten tag is loud rather than a leak.

**The privacy test.** A scenario opens a known image and types known
text; the recording is searched for any run of 64 bytes from the image
file, from the decoded pixels in any byte order, from any
downsampled copy under 1/8 scale, and for the typed text and its
UTF-16; any hit fails naming the record. The test is proven by removing
one redaction and watching it fail. It runs in CI on both targets.

## Consent and controls

- Dev builds (`debug_assertions`) record by default; the top bar shows a
  recording indicator and a toggle; the indicator is a pattern in
  touché's `design/patterns.md`.
- Release builds record only after an opt-in with the retention period
  stated (a number, e.g. 7 days locally, uploaded recordings kept 30
  days), shown once and changeable in settings; the recording indicator
  shows whenever a recording is on.
- A recording is deleted at the end of retention by the app and by the
  server; a "delete my recordings" command exists on both.
- Nothing is uploaded over a connection the user has not approved;
  uploads are chunks under a stated size with a deadline each.

## Uses

- **UX review.** `cargo xtask ux-review <session>` replays into frames
  at the recorded device size and hands them with the timing and report
  records to the review agent, which judges against `design/patterns.md`
  and the UX checklist (impl-shell's work); problems become slices.
- **Scripted scenarios per PR.** A scenario is a recording's `Input`
  records with no snapshot, run against the built app under Xvfb and in
  headless Chromium; the result is a fresh recording judged the same
  way.
- **Measures.** P0's behaviour measures (time to first edit, rephrases,
  immediate undos, waiting time) are queries over `Dispatch`, `Async`
  and `App` records.
- **Bug reports.** A refusal's record carries the last N frames of the
  log.

## Taken from where

- **rrweb**: the full-snapshot plus incremental-mutation model with
  stable node ids, periodic checkouts (`checkoutEveryNth`/`Nms`), input
  records with timestamps, throttled mouse moves, replay by
  reconstruction on a frame-clock timer with synchronous seeking, and
  the masking options (`maskAllInputs`, off by default; `maskTextSelector`;
  `blockClass` giving a same-size placeholder; no option to block
  images). Differs:
  vapors records its own tree rather than the DOM, so the mutation
  stream is the incremental update's own dirty list and replay uses the
  real painter; masking is by element tag at the source rather than by
  selector at serialisation, so a component cannot forget it.
- **Sentry Session Replay** masks all text and inputs and blocks media
  by default; **PostHog** on the web masks inputs only (passwords
  always). vapors inverts Sentry's default for UI strings (recorded) and
  makes user content the tagged exception, because UI strings are what
  the review needs and user content is marked by the element.
- **Chromium's tracing** for per-phase timings.

## Deliberately left out

- Video or pixel capture of real frames; replay paints from the tree.
- Recording the engine's GPU work beyond timings.
- Heat maps and analytics dashboards: the records are queryable; the
  tool is touché's.
- Recording of other windows, the clipboard, or the file system.

## Oracles

| Part | Oracle |
|---|---|
| Replay fidelity | A scenario recorded and replayed: the replayed frames equal the live frames pixel for pixel except inside tagged user-content boxes, where they equal the tone tile; checked on this machine's lavapipe in the `gpu` tier |
| Mutation completeness | Replay from the first snapshot plus mutations equals a snapshot taken at the end (the same shape as S7's differential test) |
| Privacy | The byte-search test above, on both targets, proven by breaking a redaction |
| Format | A recorded session from the web build replays natively and the reverse; a record with an unknown tag is refused naming the offset |
| Retention | A stepped clock test deletes at the boundary |
| Overhead | Recording on adds under a stated fraction to phase 7 in the stress demo, measured and recorded |

## Slices

| Slice | Delivers | touché use |
|---|---|---|
| C1 recorder core | Records, the format, snapshot and mutations, timings and reports, local storage, the indicator | Dev builds record; P0 timing data from the Pixel |
| C2 redaction | Tags, the tone tile, text classes, the privacy test | A recording Tom can hand to the review agent |
| C3 replay | `xtask replay` to frames with the real painter | UX review from recordings |
| C4 scenarios | Scripted input replayed against a build, both targets | Phone and desktop walkthroughs per PR |
| C5 upload and consent | Opt-in, retention, chunks through a `command` | Recordings from people who are not Tom |
| C6 measures | Queries for P0's behaviour measures | The UX scorecard |
