# 7. Platform services

Status: design, 2026-10-25. Clipboard, storage, network, key store,
file pickers, menus, timers, the system theme. The rule Tom set: app
code reaches these only through interfaces the platform implements;
`web_sys` and the native crates (arboard, rfd, keyring, muda) appear
only in platform modules, checked by a lint with named exceptions in
`.config/factory.toml`.

## The model

**One trait per service, one answer shape.** Each service is a small
trait in `vapors-platform` (or a `vapors-services` crate if the
platform crate's size asks for it), implemented once for native and
once for the web, with a recording fake for tests. App code receives
the implementations through `Platform`, which it already gets with
every event, and from scripts through a context the runtime provides.
Nothing in app code is `cfg(target_family = "wasm")`.

**Every answer is one of three states.** A service call resolves,
reports absence, or reports that it could not reach the thing. One
type, `Outcome<T>`: `Resolved(T)`, `Absent`, `Unreachable(Reason)`.
It is the shape touché's `KeyStore` already uses (R2, where the
variants are named `Found`, `Absent`, `Unavailable`; they are renamed
when the type moves into vapors). A caller that gets `Unreachable`
tells the user and stops; it never treats it as `Absent`. This is the
rule "never take something away from the user on the third", and area
8's remote functions return the same type.

**Asynchronous by shape, everywhere.** The web answers through
promises; native dialogs block another thread. So every service call
that can wait returns a future, and area 1's async model (`await` in
scripts, drop-to-cancel, a deadline on every wait) is how app code
consumes it. The existing `pick_file`, which answers with a later
`Event::FileOpened`, is kept for the image app and wrapped as a future
for components.

**Deadlines.** Every wait that is not the user's has a deadline from a
measurement, named in the code: a clipboard read, a storage
transaction, a key store lookup. Reaching it is `Unreachable` with the
deadline in the message. The user's own waits (a picker open, a
permission prompt) have none, and the call says which kind it is.

## Services

| Service | Native | Web | Three states | Notes |
|---|---|---|---|---|
| Clipboard | arboard: text and images | `navigator.clipboard` read and write, `ClipboardItem` for images; `paste` events for Ctrl/Cmd+V | `Absent` when empty or no readable type; `Unreachable` when permission is denied or the activation has lapsed | Read needs user activation on the web; the API is `read_text`, `read_image`, `write_text`, `write_image`. Each says which activation it needs, and a call outside one is refused before asking the browser |
| Key-value storage | A JSON file under the platform config directory, written atomically | `localStorage` for small values; IndexedDB for anything over a stated size or binary | `Absent` on a missing key; `Unreachable` when the backend fails or the quota is exceeded | Namespaced by app name; touché's profile and keys use it. A value whose shape the caller does not recognise is refused by the caller, naming the key |
| File storage | The project folder (`document.json`, `tiles/`) | OPFS with the same layout | As above | Area 8's F-remote sync reads the same layout |
| Key store | `keyring` (OS keychain) | storage with a stated warning: the browser's storage is the key store | `Absent` when no key; `Unreachable` when the keychain daemon is not reachable | Already built in touché R2 as `KeyStore`; moves into vapors with its tests |
| Network | reqwest with streaming | `fetch` with the body read from its `ReadableStream` | A response, `Absent` for 404 where the caller says absence is a valid answer, `Unreachable` for connection, TLS, timeout, CORS | Already built in touché R3 as `Transport`; moves into vapors under area 8. One timer rule: tokio natively, gloo-timers on the web |
| File pickers | rfd (XDG portal on Linux) | `<input type=file>` with `accept`; the File System Access API where present for save | `Absent` when cancelled; `Unreachable` when no dialog can open | Built; the save picker is new |
| Share and download | Reveal in file manager | `navigator.share` with files, else a download link | `Absent` when the user dismissed the sheet | touché M4b's Export rules move here |
| Menus and context menus | muda for the menu bar; in-app context menu | In-app menu bar and context menu drawn by vapors | n/a | Area 3 owns the context menu as a component; this row is the native menu bar only |
| Timers | The event loop's timer | `setTimeout`/`requestAnimationFrame` through the frame loop | n/a | Area 2 owns the scheduler; this is the one timer primitive it is built on |
| System theme and motion preference | winit theme events; `prefers-reduced-motion` has no winit signal, read per OS | `matchMedia` listeners | n/a | Fed into `vapors-css`'s `Environment`; a change restyles |
| Connectivity | A probe request on demand | `navigator.onLine` plus request outcomes | n/a | touché's indicator (patterns.md) is built on request outcomes; the platform only exposes them |
| Opening URLs | `open` crate | `window.open` with user activation | `Unreachable` when blocked | Links in chat and About |
| Notifications | None scheduled | None scheduled | | |

## Taken from where

- **Trait per service with a fake:** every GUI toolkit does this;
  the specific shape (small traits, recorded fakes, three states) is
  touché's R2 `KeyStore` and R3 `Transport`, which are the reference
  implementations and move into vapors rather than being rewritten.
- **Deadline on every wait:** the touché and vapors rule, not a
  framework's.
- **Dioxus and Tauri** expose platform services as plugins or
  JavaScript bindings; vapors does not, because a plugin surface would
  be a second way to reach the platform. **Leptos and Sycamore** leave
  services to `web_sys` in app code, which the lint forbids here.

## Deliberately left out

- A general "run JavaScript" escape hatch on the web. Anything the
  platform needs is a named service.
- Service workers and offline caching of the app itself: touché's
  update notice (M7c) works without one; area 8 states when one would be
  wanted.
- Platform-native menus on the web: the in-app menu is the same on both,
  so the oracle is the same.
- Mobile services (Keystore, share intents, photo library): the trait
  shapes leave room; nothing is scheduled.

## Oracles

| Part | Oracle |
|---|---|
| Clipboard on the web | Headless Chromium with `--enable-features=ClipboardAPI` and a granted permission: write then read round trip, image round trip, a read without activation refused with the stated reason |
| Clipboard natively | Xvfb with `xclip` as the other party (touché's planned native paste slice), one test per direction |
| Storage | A fake storage that fails on the Nth write, to prove `Unreachable` surfaces; quota exceeded in Chromium through a filled origin |
| Key store | The fake and, natively, a `keyring` mock backend; the real keychain by hand once per platform |
| Network | A stand-in server on a second origin (touché's `check_ask` already has one): a 200, a 404, a connection refused, a slow body past the deadline, each mapped to its state |
| Pickers | No automated oracle for the native dialog (the portal is a user interaction); the web `<input type=file>` is driven by Chromium's file chooser interception in the page-contract script |
| Lint | A test that greps every crate but the platform modules for `web_sys`, `arboard`, `rfd`, `keyring`; proven by adding one use and watching it fail |

## Slices

| Slice | Delivers | touché use |
|---|---|---|
| P1 | The service traits, the three-state answer type, the fakes, the lint | The guard check impl-shell is adding lands against a real list |
| P2 | Clipboard: text and image, both targets, activation rule | Phone Paste and native Ctrl/Cmd+V, both feeding touché's open path |
| P3 | `KeyStore` and `Transport` moved from touché with their tests | touché's runtime loses its own platform code |
| P4 | Storage: key-value and file storage with the shared layout | The profile, keys and later documents in one place |
| P5 | Save picker, share and download | Export's three paths on one API |
| P6 | Theme and motion preference into `Environment` | `prefers-reduced-motion` honoured by area 6 |
