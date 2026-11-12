# Native versions of the web platform's APIs

Design only, for after the performance finish line
(`docs/plans/perf-finish-line.md`). Nothing here is built, and nothing
here changes current work. Read from `origin/main` at 90d1c8a
(2026-11-12) and from touché `main` at ea2bb08. Every number names its
source; a number this document does not have says "to be measured".

Tom's phrase was "native versions of other web APIs": vapors offers
app code the web platform's APIs through one Rust API that works on
the web, backed by the browser, and natively, backed by the operating
system. This plan builds on `docs/design/07-platform-services.md`
(2026-10-25), which set the model (one trait per service, three-state
answers, futures, deadlines from measurement, no `web_sys` in app
code) and listed eleven services. It extends the list to what Tom
named (fetch and HTTP, WebSocket, storage, files and the File System
Access API, clipboard, drag and drop, notifications, geolocation,
camera, microphone and audio, share, workers and timers, crypto,
permissions), and for each gives the Rust shape, the web and native
backings per OS, what touché and vapors already have, the failure
states, how it is tested on the dev machine, and the practice it is
measured against.

Rules that bind every row (Tom, 2026-10-24 to 2026-11-12;
`docs/design/ARCHITECTURE.md` invariant 7; `~/.claude/CLAUDE.md`
"Failure must be loud"):

- one API for app code, with no `cfg(target_family = "wasm")` and no
  `web_sys`, `arboard`, `rfd`, `keyring` or `reqwest` outside the
  platform modules (design 07's lint);
- every answer is one of three states, `Resolved`, `Absent`,
  `Unreachable(reason)`; a caller never turns the third into the
  second, and nothing is taken from the user on the third;
- every wait that is not the user's has a deadline from a measurement,
  named in the code; reaching it is `Unreachable`;
- an input whose shape is not recognised (a stored value, a response
  body, a dropped file) is refused naming the key, field or file;
- no escape hatch: no "run JavaScript" call, no raw handle to the
  browser or the OS; a need is a named service.

## Contents

1. What exists: vapors, touché, the designs
2. The model, restated and extended
3. The services, one by one
4. Testing on the dev machine
5. Existing practice
6. Slices
7. Decisions for Tom
8. Risks

## Summary

- Design 07 already decided the shape; nothing of it is built in
  vapors (`Outcome<T>` does not exist in `crates/`, checked by grep at
  90d1c8a). touché has built four of the services itself, in exactly
  that shape, as the design intended it to: a key store in three
  states over the OS keychain and localStorage, a streamed HTTP
  transport over reqwest and `fetch` with measured deadlines, an
  IndexedDB profile store with measured deadlines under a Web Lock,
  and a deadline rule racing every wait against a timer (section 1).
  vapors itself has the clipboard read on a worker thread with a 3 s
  deadline, the file picker on both targets, a page timer as a future,
  the waker and a monotonic clock.
- The extension adds eight services design 07 did not list
  (WebSocket, drag and drop, geolocation, camera and microphone, audio,
  share as a service of its own, crypto, permissions) and makes two
  decisions explicit: workers are design 01's `offload`, never a raw
  worker API; and a service a target cannot back answers
  `Unreachable(NotOnThisTarget)` at the call, with the reason, so the
  app shows it instead of a missing feature (section 2).
- Testing: every service has a recorded fake and a two-state fault
  injector; the browser side runs in headless Chromium 155 on this
  box with DevTools permission grants; the native side runs under
  Xvfb with `xclip`, the running gnome-keyring, PipeWire's null
  devices and a private session bus from `dbus-run-session` for the
  D-Bus services; the portal is broken here, which is itself a fixture
  (section 4). Camera, geolocation, share and notifications have no
  automated native oracle on this box; a real device is the check.
- Practice: Deno is the nearest in intent (the web's APIs on a native
  runtime), Capacitor in shape (one API, a web and a native
  implementation per plugin), Tauri and React Native in the native
  module pattern; each differs in a way section 5 states.
- Fourteen slices (section 6), ten decisions for Tom (section 7).

## 1. What exists

### 1.1 In vapors (`crates/platform`, 7,080 lines)

| Piece | What it does | File |
|---|---|---|
| Clipboard, native | `Clipboard` trait; `SystemClipboard` through arboard (X11 and Wayland data-control); image and text; three answers each; a read runs on a worker thread with `DEADLINE` 3 s, and one past it is left running because arboard cannot cancel | `clipboard.rs`, `clipboard_reader.rs`, `clipboard_text.rs` |
| Paste key | Ctrl/Cmd+V detected on the key event and turned into `Event::Paste`; the web page reads its clipboard through the `paste` event itself | `paste_key.rs` |
| File picker | `Platform::pick_file(&[FileKind])`: rfd on its own thread natively (XDG portal on Linux, bounded by libdbus's 25 s, measured here with no portal: 25.1 s); a hidden `<input type=file>` on the web; the file arrives as `Message::FileChosen`, then `Event::FileOpened` | `picker.rs`, `picker_web.rs`, `file_kind.rs` |
| Timer, web | `Timer::after(Duration)` as a future over `setTimeout`, cleared on drop; `page_hidden()` | `timer_web.rs` |
| Waker | `Platform::waker()`: a frame request from a thread or a future; `LoopClosed` when the loop is gone | `waker.rs` |
| Clock | `Instant` on both targets (`web-time` on the web) | `clock.rs` |
| Status and errors | `report_status`, `report_error` to stdout or `#status`/`#error` | `report.rs`, `docs/web.md` |
| Tasks | GPU setup and the picker end by sending a `Message`; the web spawns on the browser's loop | `tasks.rs` |

Not in vapors: network, storage of any kind, a key store,
notifications, geolocation, media, share, crypto, a save picker, drag
and drop from outside the window, `Outcome<T>`.

### 1.2 In touché (`crates/runtime`, `crates/web`, `crates/app`)

Built to design 07's shape before vapors had it, and the reference
implementations the design says move into vapors:

| Service | touché's implementation | Three states and deadlines | File |
|---|---|---|---|
| Key store | `KeyStore` trait: `read` answers `KeyAnswer::{Present, Absent, Unreadable(String)}`; `OsKeychain` through `keyring-core` natively; `WebKeyStore` over a `KeyValueStorage` trait, `BrowserStorage` on localStorage, one item per provider (`touche.anthropic.key`); the rule never reads the environment when the keychain is unreachable | three states by construction; no wait (both stores are synchronous) | `runtime/src/key.rs`, `key_os.rs`, `key_web.rs` |
| HTTP, streamed | `Transport` trait: one POST, status, body as chunks; `ReqwestTransport` (rustls, tokio) natively, `FetchTransport` (`fetch`, `ReadableStream`, `AbortController`, `redirect: "manual"`) on the web; redirects never followed | `Deadlines { connect 10 s, first_byte 60 s, idle 60 s }`, chosen and documented in `http.rs`; a 3xx and a non-2xx refused by name; a redirect's origin reported | `runtime/src/transport*.rs`, `http.rs` |
| Deadline rule | `timer::within(limit, work)`: race against tokio's sleep natively, gloo-timers on the web; the work is polled first | the one rule for every wait | `runtime/src/timer.rs` |
| Profile storage, web | IndexedDB database `touche`, store `profile`, JSON under one key, every change under the Web Locks API lock `touche-profile`; a profile of another version refused by name with the backup path, never overwritten; session id in sessionStorage | measured by `check_ask` (headed Chromium 155 on Xvfb, 4 runs, 2026-10-23): open at most 59 ms, transaction 12 ms, lock wait 10 ms; deadlines 5 s open, 5 s transaction, lock = holder's bound plus 1 s | `web/profile_db.js`, `web/src/profile_web.rs` |
| Profile storage, native | `~/.config/touche/profile.json` (or `TOUCHE_PROFILE`), versioned | a file; refused by name when its version differs | `runtime/src/learn/` |
| File open | vapors' picker; `drop.js` for drag and drop onto the page (`dragover`, `drop`, `DataTransfer`) | a dropped file of an unknown kind refused by name | `web/drop.js` |
| Paste | the page's `paste` event feeding the open path; native Ctrl/Cmd+V through vapors | | `web/src/`, vapors `paste_key.rs` |
| Microphone | `getUserMedia` and `MediaRecorder` for dictation; the recording posted to OpenAI's transcription under a deadline | `touche_transcribe_deadline_ms`; the key never logged | `web/dictation.js`, `web/src/dictation_web.rs` |
| Realtime voice | WebRTC (`RTCPeerConnection`): a client secret minted with the key, the browser's SDP offer posted; events, cost and limits decided in Rust | per-session limits in `realtime/limits.rs` | `runtime/src/realtime/`, `web/` |
| Worker | a PNG encode in a Web Worker (`encode_worker_boot.js`) | | `web/encode*.js` |
| Page state | `matchMedia` (display mode, colour scheme), `visibilitychange`, `history` | | `web/*.js` |

So touché has, on the web, fetch, localStorage, sessionStorage,
IndexedDB, Web Locks, file input, drag and drop, paste, microphone
capture, WebRTC, a worker, media queries and visibility; natively,
reqwest, the keychain, a JSON file, rfd and arboard through vapors.
Each is written against the service it needs, in Rust or in a short
page script, with its own deadline. None is a vapors service yet, so
a second app would write them again.

### 1.3 In the designs

- **Design 07** (platform services): the model (section 2 restates
  it), a table of eleven services, oracles per service, slices P1 to
  P6. Its network row is touché's `Transport`; its key-store row is
  touché's `KeyStore`.
- **Design 01** (reactivity and async): `await` in three places, drop
  to cancel with a `Cancel` token for off-thread work, `offload` as
  the one off-thread call, `<boundary>`; "every wait has a deadline".
  Every service future here is consumed through it.
- **Design 02** (scheduler): tasks and microtasks are a phase; a
  worker pool on the web with structured clone, a thread pool
  natively; render-in-a-worker as a web-only option.
- **Design 08** (remote functions): `Outcome<T>` shared with the
  services; `fetch` reached only through the transport.
- **Design 03**: drag within the window (pointer capture) is events,
  not a service; drag from outside the window (a file from the
  desktop) is this plan's.

## 2. The model, restated and extended

Design 07's model holds. Restated in one place so each row of section
3 can refer to it:

1. **One trait per service**, in `vapors-platform` (or
   `vapors-services`, decision D1), implemented once for native and
   once for the web, plus a recording fake. App code receives them
   through `Platform` (already passed with every event) and from
   scripts through a context the runtime provides.
2. **`Outcome<T>`**: `Resolved(T)`, `Absent`, `Unreachable(Reason)`.
   `Absent` is an answer (no key, empty clipboard, user cancelled);
   `Unreachable` is the lack of one (denied, timed out, no backend,
   not on this target, a value of an unrecognised shape). touché's
   `KeyAnswer::{Present, Absent, Unreadable}` is this type with other
   names and is renamed when it moves.
3. **Futures, with deadlines.** Every call that can wait returns a
   future; the deadline is a `Duration` constant beside the call with
   a comment naming its measurement (as touché's `Deadlines` and
   `profile_db.js` do); the user's own waits (a dialog, a permission
   prompt) have none, and the call's doc says which kind it is.
4. **Activation.** Several web APIs run only inside a user gesture
   (clipboard read, the file pickers, share, fullscreen, `window.open`,
   notifications permission). A service call that needs activation
   says so in its signature by taking an `Activation` token that only
   an event handler can mint (design 03's dispatch hands it to the
   listener). A call outside one is refused at compile time, not
   by the browser at run time.
5. **Permissions as a service.** `Permissions::query(Capability)`
   answers `Granted`, `Denied`, `Prompt` or `NotOnThisTarget`; a
   service that needs a permission asks for it itself when first used
   and reports `Unreachable(Denied)` with the capability named. The
   app never asks for a permission it will not use at once (the
   browsers' own guidance, and the rule that keeps the prompt next to
   the thing it enables).
6. **A target without a backing.** Where an OS has no way to back a
   service (push notifications natively, a vibration motor on a
   desktop, geolocation on a Linux box with no geoclue agent), the
   implementation exists and answers `Unreachable(NotOnThisTarget)`
   with the reason, so the app's `match` arm shows it. The
   alternative, no implementation on that target, makes the app's
   code `cfg`-dependent, which rule one forbids (decision D2).
7. **Shapes refused.** A stored value, a response body, a dropped
   file, a message from a worker: each has a declared shape (a `serde`
   type with a version where it persists), checked on the way in;
   a mismatch is `Unreachable(Shape)` naming the key or field, never a
   default. touché's profile refusal is the model.
8. **No raw handles.** No `web_sys::Window`, no `winit::Window`, no
   file descriptor crosses to app code. A service that would need one
   is a missing service.
9. **Oracles.** Each service names what proves it (section 4): the
   browser through DevTools where the semantics are the web's, a
   stand-in native service where the OS has one, a fake with fault
   injection everywhere.

## 3. The services

Per service: the Rust shape (a sketch; names are the slice's to
settle), the web backing, the native backing per OS, what exists,
failure states, and the test on the dev machine. "Mobile" means iOS
and Android, which are untested on any vapors target (positioning
record) and listed so the trait leaves room.

### 3.1 Fetch and HTTP

```rust
pub trait Http {
    fn send(&self, request: Request, deadlines: Deadlines) -> impl Future<Output = Outcome<Response>>;
}
pub struct Response { status: u16, headers: Headers, body: Body }   // Body: a stream of chunks, read under the idle deadline
```

| | |
|---|---|
| Web | `fetch` with `ReadableStream` and `AbortController`, `redirect: "manual"`: touché's `FetchTransport` moved |
| Linux, macOS, Windows | reqwest with rustls and ring on the loop's tokio runtime: touché's `ReqwestTransport` moved; connect deadline on the client |
| Exists | touché `transport.rs`, `transport_web.rs`, `transport_native.rs`, `http.rs` (deadlines, SSE framing in `sse.rs`) |
| States | `Resolved` for 2xx; `Absent` for 404 (design 08: only a server's answer); `Unreachable(Status)` for other 4xx and 5xx; `Unreachable(Redirect(origin))`: never followed; `Unreachable(Connect/FirstByte/Idle)` at the deadlines; `Unreachable(Cors)` on the web; `Unreachable(Shape)` when a body fails its schema |
| Differences from the web | no cookies, no cache, no redirect following, no credentials mode: each would be a second way the key could travel; a header the app did not set is never added except by the browser itself |
| Test | the stand-in server on a second origin touché's `check_ask` runs (a 200, a 404, a refused connection, a slow body past the deadline); headless Chromium for CORS and `opaqueredirect`; a fake transport with a scripted chunk sequence |

WebSocket, server-sent events and streaming bodies are 3.2.

### 3.2 WebSocket and streams

```rust
pub trait Sockets {
    fn connect(&self, url: Url, deadline: Duration) -> impl Future<Output = Outcome<Socket>>;
}
impl Socket { fn send(&self, Message) -> impl Future<Output = Outcome<()>>; fn next(&mut self) -> impl Future<Output = Outcome<Message>>; fn close(self, reason) }
```

| | |
|---|---|
| Web | `WebSocket`: `open`, `message`, `error`, `close` events into the future's waker; binary as `ArrayBuffer` |
| Native | `tokio-tungstenite` over rustls, or reqwest's websocket upgrade (one client, decision D4) |
| Exists | nothing; touché's realtime voice uses WebRTC, not WebSocket (3.9); SSE is a streamed HTTP body (3.1) |
| States | `Resolved(Socket)`; `Unreachable(Connect)` at the deadline; `Unreachable(Closed(code, reason))` from `next` after the peer closes (the close is the last message, not an error); `Absent` is never used: a socket has no "none" |
| Differences from the web | the web's `WebSocket` has no connect deadline and a browser-chosen buffer; the service has both stated; reconnection with backoff is design 08's live query, not the socket's |
| Test | a stand-in server (tungstenite in the test) on the loopback: echo, a server close with a code, a stalled handshake past the deadline; the web side against the same server from headless Chromium |

### 3.3 Key-value storage (localStorage, the preferences)

```rust
pub trait KeyValue {
    fn get<T: DeserializeOwned + Versioned>(&self, key: &Key) -> impl Future<Output = Outcome<T>>;
    fn set<T: Serialize + Versioned>(&self, key: &Key, value: &T) -> impl Future<Output = Outcome<()>>;
    fn remove(&self, key: &Key) -> impl Future<Output = Outcome<()>>;
}
```

| | |
|---|---|
| Web | `localStorage` for values under a stated size (design 07: "small values"; the size to be chosen from the browsers' per-origin quotas, 5 MB in Chromium), JSON text; synchronous, wrapped as a ready future |
| Linux | a JSON file per app under `$XDG_CONFIG_HOME/<app>/`, written atomically (write, fsync, rename): touché's profile file |
| macOS | the same under `~/Library/Application Support/<app>/` |
| Windows | the same under `%APPDATA%\<app>\` |
| Exists | touché `KeyValueStorage` (web, strings), `learn::ProfileStore` (native file), both versioned |
| States | `Absent` on a missing key; `Unreachable(Blocked)` in a private window or with storage blocked (the accessor throws); `Unreachable(Quota)`; `Unreachable(Shape(key, version))` when the value's version or shape is not the caller's: refused with the way to back it up and start again, never overwritten (touché's rule) |
| Differences from the web | typed and versioned values, not strings; namespaced by app and key, so two vapors apps on one origin never collide (a prefix, as `touche.anthropic.key` does) |
| Test | a fake with a scripted failure on the Nth write; the file store against a read-only directory and a full tmpfs (`Unreachable`, not a silent loss); the web store in headless Chromium with storage blocked through DevTools (`Emulation` and a `--disable-local-storage`-style flag, to be chosen) |

### 3.4 Document storage (IndexedDB-like, OPFS, the project folder)

The web's IndexedDB is a transactional key-object store with indexes;
the Origin Private File System is a per-origin directory of files
with synchronous access handles in workers. touché needs both shapes:
a versioned JSON document under a lock (IndexedDB) and a folder of
tiles (OPFS or a project directory, `DESIGN.md` "Persistence").

```rust
pub trait Documents {
    fn open(&self, name: &str) -> impl Future<Output = Outcome<Store>>;
}
impl Store {
    fn read<T: Versioned>(&self, key: &Key) -> impl Future<Output = Outcome<T>>;
    fn update<T: Versioned>(&self, key: &Key, change: impl FnOnce(Option<T>) -> T) -> impl Future<Output = Outcome<()>>;  // one transaction under one lock
}
pub trait Files {   // the app's own directory
    fn read(&self, path: &RelPath) -> impl Future<Output = Outcome<Vec<u8>>>;
    fn write(&self, path: &RelPath, bytes: &[u8]) -> impl Future<Output = Outcome<()>>;   // atomic
    fn list(&self, dir: &RelPath) -> impl Future<Output = Outcome<Vec<Entry>>>;
    fn remove(&self, path: &RelPath) -> impl Future<Output = Outcome<()>>;
}
```

| | |
|---|---|
| Web | `Documents`: IndexedDB, one object store per `Store`, each `update` under a Web Locks lock named after the store, one transaction: touché's `profile_db.js` generalised; `Files`: OPFS, `FileSystemDirectoryHandle` from `navigator.storage.getDirectory()`, writes through a `FileSystemWritableFileStream` (atomic on close) or a sync access handle in a worker for tiles |
| Native | `Documents`: a JSON file per key under the app's data directory with an advisory file lock (`fs2`/`std` `try_lock` on 1.89+) held for the update, atomic rename; `Files`: the same directory tree, atomic rename per write; the project folder when the app names one |
| Exists | touché `profile_db.js` with the measured deadlines (open 5 s, transaction 5 s, lock the holder's bound + 1 s) and `profile_web.rs`; the native file store |
| States | `Absent` for a missing key or path; `Unreachable(Locked)` past the lock deadline, naming the other holder's work ("close other touché tabs"); `Unreachable(Quota)`; `Unreachable(Shape(version))` as 3.3; `Unreachable(Blocked)` when IndexedDB or OPFS is unavailable (Safari private mode, some embedded contexts) |
| Differences from the web | no indexes, cursors or key ranges: a `Store` is keys to versioned values under a lock, which is what touché uses and what Capacitor's Preferences and Tauri's `store` plugin also are; an app that needs queries writes them over `Files` (decision D3) |
| Test | the deadlines re-measured by the moved `check_ask` steps; two tabs in headless Chromium contending the lock (DevTools opens the page twice); a fake store with a transaction that never completes; natively, two processes contending the file lock, and a full disk |

### 3.5 File open, save and the File System Access API

```rust
pub trait Pickers {
    fn open(&self, kinds: &[FileKind], activation: Activation) -> impl Future<Output = Outcome<PickedFile>>;   // bytes and a name, as today
    fn save(&self, suggested: &str, kinds: &[FileKind], bytes: Bytes, activation: Activation) -> impl Future<Output = Outcome<()>>;
    fn open_handle(&self, kinds: &[FileKind], activation: Activation) -> impl Future<Output = Outcome<FileHandle>>;   // re-readable, re-writable
}
```

| | |
|---|---|
| Web | `open`: `<input type=file>` as today; `save`: `showSaveFilePicker` where present (Chromium, Edge), else a download through an `<a download>` (Firefox, Safari), and the call's answer says which happened; `open_handle`: `showOpenFilePicker` where present, `Unreachable(NotOnThisTarget)` elsewhere; handles can be kept in IndexedDB and re-permissioned on the next visit (`queryPermission`/`requestPermission`) |
| Linux | rfd through the XDG portal (as today; 25 s libdbus bound when the portal is absent, `picker.rs`), or GTK when the app is not sandboxed and the portal is absent (decision D5); a handle is a path |
| macOS, Windows | rfd's native dialogs; a handle is a path (and a security-scoped bookmark on a sandboxed macOS app, later) |
| Exists | vapors `pick_file` both targets; touché's Export rules (M4b) for the save paths |
| States | `Absent` when the user cancels (both targets; an old browser fires no event for a cancelled picker, which `picker_web.rs` records: the wait is the user's, so no deadline, and `Absent` arrives only when the browser says so); `Unreachable(NoDialog)` when no dialog can open (no portal, no display); `Unreachable(Denied)` for a handle whose permission lapsed; `Unreachable(Shape)` for a file the app's `FileKind` does not cover |
| Test | the web picker through DevTools' `Page.setInterceptFileChooserDialog` and `DOM.setFileInputFiles` (the page-contract script's planned route); the save path by checking the download in Chromium's download directory; natively no automated oracle for the dialog (the portal is a user interaction, and on this box the portal is failed, which is the `Unreachable(NoDialog)` fixture: measured 25.1 s); the fake for the rest |

### 3.6 Clipboard

```rust
pub trait ClipboardService {
    fn read_text(&self, activation: Activation) -> impl Future<Output = Outcome<String>>;
    fn read_image(&self, activation: Activation) -> impl Future<Output = Outcome<ClipboardImage>>;
    fn write_text(&self, text: &str, activation: Activation) -> impl Future<Output = Outcome<()>>;
    fn write_image(&self, image: &ClipboardImage, activation: Activation) -> impl Future<Output = Outcome<()>>;
}
```

| | |
|---|---|
| Web | `navigator.clipboard.readText`/`read` (Chromium asks the `clipboard-read` permission; Firefox and Safari show a paste prompt and have no Permissions API entry); `write`/`writeText`; images cross as PNG `ClipboardItem`s; the `paste` event stays the route for Ctrl/Cmd+V (design 07) |
| Linux | arboard: X11 and Wayland data-control, as today, on the worker thread with the 3 s deadline |
| macOS, Windows | arboard's NSPasteboard and Win32 backends; the same reader |
| Exists | vapors `clipboard.rs`, `clipboard_reader.rs`, `clipboard_text.rs`, `paste_key.rs`; touché's paste path on both targets |
| States | `Absent` when empty or no readable type; `Unreachable(Denied)` on a refused permission or lapsed activation; `Unreachable(Timeout)` at the 3 s deadline (a Wayland owner that never answers); `Unreachable(NotOnThisTarget)` for an image write where the backend has none |
| Test | headless Chromium with `Browser.grantPermissions` for `clipboardReadWrite`: write then read, text and PNG; natively under Xvfb with `xclip` as the other party (installed here), one test per direction and type; the deadline by a stand-in owner that never answers (an X11 selection owner in the test that ignores requests) |

### 3.7 Drag and drop from outside the window

```rust
// delivered through design 03's dispatch, not a service call:
Event::DragEnter { kinds: Vec<DragKind> }, Event::DragOver { client }, Event::DragLeave, Event::Drop { items: Vec<DroppedItem> }
enum DroppedItem { File { name, bytes }, Text(String), Url(Url) }
```

| | |
|---|---|
| Web | `dragenter`/`dragover`/`drop` on the canvas with `DataTransfer`: touché's `drop.js` moved; `preventDefault` on `dragover` or the browser navigates |
| Linux, macOS, Windows | winit's `DroppedFile`, `HoveredFile`, `HoveredFileCancelled` (paths; the platform reads the bytes under a size limit the app states); text and URL drops where winit reports them (to be checked per OS at the slice) |
| Exists | touché `drop.js`; vapors nothing native |
| States | a dropped item of a kind the app did not declare is refused naming the kind and the declared list (`Event::DropRefused`), never silently ignored; a file larger than the stated limit likewise |
| Test | DevTools `Input.dispatchDragEvent` with a `DataTransfer` of files in headless Chromium; natively winit's drop needs a real window manager and a drag source, which Xvfb alone lacks: a recorded transcript (oracle kind 5 in design 11) from a desktop, replayed |

Drag within the window (a slider, a layer reorder) is pointer capture
(design 03) and not this plan.

### 3.8 Notifications

```rust
pub trait Notifications {
    fn show(&self, notice: Notice, activation: Option<Activation>) -> impl Future<Output = Outcome<NoticeHandle>>;   // title, body, optional icon
}
```

| | |
|---|---|
| Web | `Notification.requestPermission` (inside a gesture) then `new Notification`; shown only while the page can run; push (a service worker) is out, as design 07 says |
| Linux | `org.freedesktop.Notifications` over the session bus (`notify-rust`); `Unreachable(NoDaemon)` when no server answers |
| macOS | `UNUserNotificationCenter`, which needs a signed bundle with an identifier: a bare binary gets `Unreachable(NotOnThisTarget(NoBundle))`, stated |
| Windows | toast notifications through WinRT (`tauri-winrt-notification` or `windows` crate), which need an AppUserModelID: the same stated refusal without one |
| Exists | nothing in vapors or touché; touché's toasts are in-app (design patterns), which stay the first choice |
| States | `Absent` never; `Unreachable(Denied)`, `Unreachable(NoDaemon)`, `Unreachable(NotOnThisTarget(..))` |
| Test | headless Chromium with `Browser.grantPermissions` `notifications` and DevTools' notification events; Linux: a stand-in `org.freedesktop.Notifications` service on a private bus from `dbus-run-session` (installed here), asserting the call; no daemon on this box otherwise (`notify-send` is present, no server runs), which is the `NoDaemon` fixture |

### 3.9 Geolocation

```rust
pub trait Geolocation {
    fn current(&self, accuracy: Accuracy, deadline: Duration) -> impl Future<Output = Outcome<Position>>;
    fn watch(&self, accuracy: Accuracy) -> impl Future<Output = Outcome<PositionStream>>;
}
```

| | |
|---|---|
| Web | `navigator.geolocation.getCurrentPosition`/`watchPosition` with the permission prompt; secure context only |
| Linux | geoclue2 over D-Bus with an agent (the binary is here, `/usr/libexec/geoclue`; no agent on a headless box), else `Unreachable(NoProvider)` |
| macOS | CoreLocation (needs the usage string in the bundle's Info.plist; a bare binary is refused by the OS: `NotOnThisTarget(NoBundle)`) |
| Windows | `Windows.Devices.Geolocation` through the `windows` crate |
| Exists | nothing; no vapors app needs it; listed because Tom named it |
| States | `Unreachable(Denied)`, `Unreachable(Timeout)` at the stated deadline, `Unreachable(NoProvider)`, `Unreachable(NotOnThisTarget)`; `Absent` never (a position either arrives or does not) |
| Test | headless Chromium with `Emulation.setGeolocationOverride` and the permission granted; Linux: a stand-in geoclue on a private bus; a real phone over the tailnet for the prompt itself |

### 3.10 Camera, microphone and audio

```rust
pub trait Media {
    fn microphone(&self, activation: Activation) -> impl Future<Output = Outcome<AudioCapture>>;   // frames of f32 samples at a stated rate
    fn camera(&self, activation: Activation, want: CameraWant) -> impl Future<Output = Outcome<FrameStream>>;   // RGBA frames at a stated size
    fn play(&self, source: AudioSource) -> impl Future<Output = Outcome<Playback>>;
}
```

| | |
|---|---|
| Web | `getUserMedia` for both; microphone through an `AudioWorklet` (frames into a ring buffer the wasm reads) or `MediaRecorder` where the app wants an encoded blob (touché's dictation); camera through a hidden `<video>` drawn to an `OffscreenCanvas` or `VideoFrame` from `MediaStreamTrackProcessor` where present; playback through `AudioContext` |
| Linux | PipeWire (running here, with `pipewire-pulse`) through `cpal` for audio in and out; V4L2 through `nokhwa` for the camera (no `/dev/video*` on this box) |
| macOS | CoreAudio through `cpal`; AVFoundation through `nokhwa` (usage strings in the bundle required for both, as 3.9) |
| Windows | WASAPI through `cpal`; Media Foundation through `nokhwa` |
| Exists | touché `dictation.js` (`getUserMedia`, `MediaRecorder`), the realtime voice session over WebRTC (`RTCPeerConnection`, the SDP offer posted with a minted secret); nothing native |
| States | `Unreachable(Denied)`, `Unreachable(NoDevice)`, `Unreachable(InUse)`, `Unreachable(NotOnThisTarget)`; `Absent` never |
| Differences from the web | no `MediaStream` object: the service answers typed frame streams; encoding (Opus, WebM) is the app's or a later service; WebRTC itself is not a service (touché's realtime use is the page's, decision D6) |
| Test | headless Chromium with `--use-fake-device-for-media-stream` and `--use-fake-ui-for-media-stream` (a synthetic tone and a test pattern), permission granted; Linux: PipeWire's null sink and source under the running daemon for audio; camera: `v4l2loopback` is not loaded here and loading a module is root's, so the camera has no native oracle on this box (a real device) |

### 3.11 Share

```rust
pub trait Share {
    fn share(&self, payload: SharePayload, activation: Activation) -> impl Future<Output = Outcome<()>>;   // text, url, files
}
```

| | |
|---|---|
| Web | `navigator.share` with files where `canShare` says so (Chromium, Safari; not Firefox desktop); else `Unreachable(NotOnThisTarget)` and the app offers the save path (3.5) instead; the download link is the save path, not share |
| Linux | no share sheet; "reveal in file manager" through the portal's `OpenURI` or `xdg-open` on the parent directory, as design 07 says, stated as what it is |
| macOS | `NSSharingServicePicker` |
| Windows | `DataTransferManager` (needs a window handle and WinRT) |
| Exists | touché M4b's Export rules (web share, download, native reveal), in touché |
| States | `Absent` when the user dismisses the sheet; `Unreachable(NotOnThisTarget)`; `Unreachable(Shape)` for a file type the target cannot share |
| Test | headless Chromium cannot show a share sheet: the call's branch into `canShare` is tested with a stubbed `navigator.share` through DevTools `Runtime.addBinding`; a real phone for the sheet; natively the reveal is checked by the stand-in portal |

### 3.12 Workers and timers

Timers are design 02's: one timer primitive the scheduler owns,
`Timer::after` on the web today and the loop's timer natively;
`timer::within` from touché is the deadline rule over it. App code
gets `sleep(Duration)` and `within(Duration, future)` and nothing
else; `setInterval` is a loop over `sleep` in a future that drop
cancels.

Workers are design 01's `offload`: a named free function taking owned
data, run on a thread pool natively and a worker pool on the web with
structured clone, with a `Cancel` token. There is no `Worker` type,
no `postMessage`, no shared memory in app code (decision D7). touché's
PNG-encode worker becomes an `offload` function.

| | |
|---|---|
| States | `Unreachable(Cancelled)` when the token fired; `Unreachable(Panicked)` with the message when the function panicked (caught at the task boundary, design 01); a result for a dropped future is discarded with a debug log |
| Test | design 02's threading test (a dropped future, the worker terminated and replaced on the web, the step count pinned natively); `within` with a manual executor and a stepped clock |

### 3.13 Crypto

```rust
pub fn random_bytes(buf: &mut [u8]) -> Outcome<()>;   // the CSPRNG
pub fn digest(algorithm: Digest, bytes: &[u8]) -> [u8; N];   // SHA-256, SHA-512; pure Rust, no outcome needed
pub trait Keys { fn generate / sign / verify / encrypt / decrypt  ... }   // later, when an app needs it
```

| | |
|---|---|
| Web | `crypto.getRandomValues` through `getrandom`'s wasm backend; digests in pure Rust (`sha2`), which runs on both targets and needs no `SubtleCrypto` promise; `SubtleCrypto` only for a non-extractable key, when one is needed |
| Native | `getrandom` (the OS's CSPRNG), `sha2`, `ring` (already linked through rustls) |
| Exists | touché uses `crypto` in four places in `crates/web` (to be read at the slice); the key rule hashes nothing |
| States | `random_bytes`: `Unreachable(NoEntropy)` (an OS failure, never in practice); the rest are pure functions |
| Differences from the web | the API is a Rust crate's, not `SubtleCrypto`'s promise-and-`CryptoKey` shape; the service exists so that app code never reaches `web_sys::Crypto` |
| Test | known-answer tests for the digests; `random_bytes` twice differs; the wasm clippy tier proves it compiles for the web |

### 3.14 Permissions

```rust
pub trait Permissions {
    fn query(&self, capability: Capability) -> impl Future<Output = PermissionState>;   // Granted, Denied, Prompt, NotOnThisTarget
}
```

| | |
|---|---|
| Web | `navigator.permissions.query` for the names the browser has (`geolocation`, `notifications`, `clipboard-read`, `camera`, `microphone` in Chromium; Firefox and Safari lack several, which answer `Prompt`); requesting is each service's own call |
| Linux | portals and D-Bus services have no query without a request; the state is learned by the request: `query` answers `Prompt` |
| macOS | `AVCaptureDevice.authorizationStatus`, `CLLocationManager.authorizationStatus`, `UNUserNotificationCenter.getNotificationSettings`: real answers |
| Windows | the privacy settings through WinRT capability APIs: real answers for camera, microphone, location |
| Exists | nothing |
| States | its answer is a four-state enum, not an `Outcome`: a query cannot fail, only answer `Prompt` when it does not know |
| Test | headless Chromium with `Browser.grantPermissions` and `Browser.resetPermissions`, each state read back; the fake for native |

### 3.15 Rows design 07 has that this plan keeps as they are

Key store (touché's `KeyStore`, three states, moves in P3 of design
07; `localStorage` on the web with the stated warning); menus and
context menus (muda, in-app); system theme and motion preference into
`Environment`; connectivity from request outcomes; opening URLs
(`open` crate, `window.open` under activation).

## 4. Testing on the dev machine

What this box is (checked 2026-11-12 with `command -v` and
`systemctl --user`; `~/.claude/.../memory/dev-machine-gpu.md` for the
GPU): Debian 13, headless, no display server; Xvfb and `xclip`
installed; Chromium 155 at `/usr/bin/chromium`; a user session D-Bus
running with `gnome-keyring-daemon`, `at-spi`, PipeWire and
`pipewire-pulse` active; `xdg-desktop-portal` and its GTK backend
loaded and **failed** (no display), `xdg-document-portal` active;
`geoclue`, `notify-send` and `dbus-run-session` present with no
notification daemon or geoclue agent running; no `/dev/video*`;
`/dev/snd` present; node and npm present; the tailnet serves builds
to Tom's Pixel 9 Pro (`memory/tailscale-web-serving.md`).

| Service | Web oracle (headless Chromium 155, `env -u DBUS_SESSION_BUS_ADDRESS`, DevTools over a port) | Native oracle on this box | Fault injection (every target) |
|---|---|---|---|
| HTTP, WebSocket | the stand-in server on a second origin (touché `check_ask`); CORS, `opaqueredirect` | the same server on the loopback | a fake transport scripted per chunk; the deadline with a stalled server |
| Key-value, documents | storage blocked, quota, two tabs on one lock | a read-only directory, a full tmpfs, two processes on one file lock | the fake's Nth write fails |
| Pickers | `Page.setInterceptFileChooserDialog`, `DOM.setFileInputFiles`; the download directory for save | none for the dialog; the failed portal gives `NoDialog` after 25.1 s (measured, `picker.rs`) | the fake answers `Absent` and `Unreachable(NoDialog)` |
| Clipboard | `Browser.grantPermissions` `clipboardReadWrite`; text and PNG round trips | Xvfb with `xclip` both directions; an X11 owner that never answers for the 3 s deadline | the fake |
| Drag and drop | `Input.dispatchDragEvent` with files | none (needs a drag source and a window manager); a recorded transcript replayed | a declared-kind mismatch |
| Notifications | `Browser.grantPermissions` `notifications`; DevTools' notification events | a stand-in `org.freedesktop.Notifications` on `dbus-run-session`'s private bus; the absent daemon is the `NoDaemon` case | the fake |
| Geolocation | `Emulation.setGeolocationOverride` with the permission granted | a stand-in geoclue on a private bus; the phone for the real prompt | the fake; the deadline |
| Microphone, camera, audio | `--use-fake-device-for-media-stream`, `--use-fake-ui-for-media-stream` (tone and pattern); the AudioWorklet's frame count pinned | PipeWire null sink and source for audio; **no camera oracle** here (no device, `v4l2loopback` needs root) | the fake; `NoDevice`, `InUse` |
| Share | a stubbed `navigator.share` through `Runtime.addBinding`; the phone for the sheet | the stand-in portal for reveal | the fake |
| Workers, timers | design 02's tests | thread pool step counts | a cancelled token; a panicking function |
| Crypto | the wasm tier compiles; known answers | known answers | none needed |
| Permissions | `grantPermissions` and `resetPermissions` read back | the fake | none needed |

Rules the tests keep (`CLAUDE.md`): every headless Chromium strips
`DBUS_SESSION_BUS_ADDRESS` (else every HTTP load waits 120 s on the
failed portal); every git-running test strips `GIT_*`; each run owns
its DevTools port and kills its process group; a tier that cannot run
fails naming what is missing, so a machine without `xclip` or PipeWire
fails the native clipboard or audio test by name rather than skipping
it. Services with no native oracle here (camera, drag and drop, share
sheets, the real dialogs) are listed as such in the service's doc and
checked on a real device: the phone over the tailnet for the web,
Tom's desktop for native, with the recorded-transcript oracle where a
session can be replayed.

## 5. Existing practice

Stated as the real degree of similarity, with the differences.

- **Deno** (docs.deno.com "Web Platform APIs", read 2026-11-12):
  `fetch`, `WebSocket`, Web Storage (`localStorage`), Web Workers,
  Web Crypto, `CompressionStream`, Cache API, `OffscreenCanvas`,
  `navigator`, with a "spec deviations" note per API. The nearest in
  intent: the web's APIs on a native runtime. The difference is the
  surface: Deno reproduces the JavaScript API exactly, exceptions and
  all, because JavaScript code must run unchanged; vapors offers Rust
  traits with the web's semantics and a three-state answer, because
  the app is Rust and the rule is loud failure in one shape. Deno has
  no browser side; its APIs are the native side only.
- **Capacitor** (capacitorjs.com/docs/apis, read 2026-11-12):
  Clipboard, Filesystem, Geolocation, Camera, Share, Http,
  Preferences, Local and Push Notifications, Network, Device, Dialog,
  Motion, Haptics and about twenty more; each plugin has a JavaScript
  API with an iOS, an Android and a web implementation, and a web
  implementation that cannot do the thing throws "not implemented".
  The nearest in shape: one API, a web and a native backing per
  service. Differences: JavaScript, mobile-first with desktop through
  Electron, and the "not implemented" throw is a two-state failure
  where vapors answers `Unreachable(NotOnThisTarget)` with the reason
  and keeps the app running.
- **Tauri plugins** (tauri-apps/plugins-workspace README, read
  2026-11-12): clipboard-manager, dialog, fs, geolocation, http,
  notification, store (key-value), stronghold, websocket, opener,
  os, updater, upload and others; each a Rust crate with a JavaScript
  API that calls it over Tauri's IPC. Differences: the app code is
  JavaScript in a WebView that already has the browser's own APIs, so
  Tauri's plugins add native ones beside them rather than giving one
  API over both; vapors has no IPC and no WebView, and the app is Rust
  on both sides. Tauri's `store` plugin and `dialog` plugin are the
  same shape as 3.3 and 3.5 and are a reference for OS details.
- **React Native modules**: native modules (Turbo Modules over JSI)
  with a JavaScript interface and per-OS implementations; community
  packages for clipboard, geolocation, camera, notifications;
  `react-native-web` is a separate project that maps the component
  layer, not the modules. Differences: JavaScript, mobile-first, no
  web backing in the module system itself.
- **Rust-wasm web wrappers**: `gloo` (`gloo-net`, `gloo-storage`,
  `gloo-timers`, `gloo-worker`) and `web-sys` are what the web half of
  each service is written with; they are the browser side only, with
  the browser's own failure shapes (a `JsValue` error).
- **Rust native crates**: `reqwest`, `tokio-tungstenite`, `arboard`,
  `rfd`, `keyring`, `notify-rust`, `cpal`, `nokhwa`, `open`,
  `directories`; each is one OS side of one service, and the service
  trait is what makes them one API.
- **Dioxus SDK**: clipboard, storage, geolocation, notifications and
  more for Dioxus apps on desktop and web through one Rust API: the
  nearest in language and shape. Differences, as far as this plan can
  state them without reading its source at a pinned version: errors
  are per-crate `Result`s rather than one three-state type, and there
  is no deadline rule; to be checked at slice S1 and the difference
  recorded.

No framework in this list has the three-state answer, the deadline
from measurement on every wait, or the activation token in the
signature; those are vapors' rules, carried from touché.

## 6. Slices

Each slice is one PR under about 400 lines (estimate), built by a
fresh agent; each check proven by breaking what it guards, watching it
fail, restoring, stated in the commit. Design 07's P1 to P6 are
absorbed and renumbered; where a row moves touché code, the touché PR
that switches touché to the vapors service follows it.

| PR | Content | Accepted when | Lines | Depends on |
|---|---|---|---|---|
| S1 | `Outcome<T>`, `Activation`, `PermissionState`, the service traits as empty modules with recording fakes, the fault injector (`fail_on(n)`), the lint that no crate outside `platform` names `web_sys`, `arboard`, `rfd`, `keyring`, `reqwest`, `gloo`; the practice note for Dioxus SDK checked | the lint fails on an added use (proven); `Outcome` has no `From<Option<T>>` (a test that it does not compile, so `Absent` is never made from `None` by accident); fakes replay a scripted answer sequence | 350 | none |
| S2 | HTTP: touché's `Transport`, `Deadlines`, `within` and SSE framing moved into `vapors-platform` as `Http`; touché points at them | touché's `check_ask` and its transport tests pass against the moved code; the stand-in server's four cases pinned in vapors | 400 | S1 |
| S3 | Key store: touché's `KeyStore` moved, `KeyAnswer` renamed to `Outcome` | touché's key tests pass on the moved code; the keychain-unreachable rule pinned (environment never read); an entry written and read back through the gnome-keyring running on this box, and `Unreachable` with no secret service on a private bus from `dbus-run-session` | 250 | S1 |
| S4 | Key-value storage: the versioned file store (atomic rename) and the localStorage store; `Shape(version)` refusal with the backup path | the fake's Nth-write failure surfaces; the full-tmpfs and read-only cases `Unreachable`; a value of another version refused naming the key, never overwritten (proven) | 400 | S1 |
| S5 | Documents: IndexedDB store under a Web Lock from `profile_db.js`, generalised to named stores; the native file-lock store; deadlines re-measured and recorded | touché's profile moves onto it; two tabs contend the lock in headless Chromium and the second waits, then succeeds; past the lock deadline `Unreachable(Locked)` names the holder | 400 | S4 |
| S6 | Files: OPFS and the app directory, atomic writes, list and remove | a tile written and read back on both targets; quota exceeded in Chromium through a filled origin surfaces as `Unreachable(Quota)` | 350 | S5 |
| S7 | Pickers: `save` (`showSaveFilePicker` else download; rfd save dialog) and `open_handle`; `Activation` on `open` | DevTools intercepts the chooser; the download appears; a call without activation does not compile (a `compile_fail` test) | 400 | S1 |
| S8 | Clipboard as a service on both targets: `navigator.clipboard` read and write with the permission; the native reader behind the trait | PNG and text round trips in Chromium with the permission granted and refused; `xclip` both directions under Xvfb; the 3 s deadline hit by a silent owner | 350 | S1 |
| S9 | Drag and drop: `Event::Drag*`/`Drop` from `drop.js` moved and winit's file drops; declared kinds; the size limit | `Input.dispatchDragEvent` with two files delivers one `Drop`; an undeclared kind yields `DropRefused` naming it; a recorded native transcript replays | 350 | S1 |
| S10 | Permissions and notifications: `Permissions::query`; `Notifications` on the web and Linux (notify-rust), `NotOnThisTarget(NoBundle)` on macOS and Windows until a bundle exists | `grantPermissions`/`resetPermissions` read back; a notification shows in Chromium (DevTools event); the stand-in D-Bus service receives the call; no daemon gives `NoDaemon` | 400 | S1 |
| S11 | Workers and timers: `sleep`, `within` and `offload` as the public surface; touché's PNG worker on `offload` | design 02's threading test; `within` under a stepped clock; touché's export encodes through `offload` on both targets | 400 | S1; design 01's offload slice |
| S12 | Microphone and audio out: `getUserMedia` through an AudioWorklet to typed frames; `cpal` over PipeWire; playback | the fake device's tone is read back with its frequency pinned (web); PipeWire null source and sink natively; `Denied`, `NoDevice` surfaced | 400 | S10 |
| S13 | Geolocation and share | geolocation override read back; stand-in geoclue; `canShare` stub; the phone check recorded in the PR | 350 | S10 |
| S14 | Camera and crypto: `nokhwa` and `getUserMedia` video to RGBA frames; `random_bytes`, digests | fake-device pattern frames hashed and pinned (web); no native oracle here, stated; known-answer tests | 400 | S12 |

Order: S1, S2, S3, S4, S5, S6, S7, S8, S9, S11, S10, S12, S13, S14.
S2 to S6 are what touché needs today; S7 to S9 are what it needs next
(export, paste, drop on the desktop); S10 to S14 are the services
with no vapors user yet, ordered by how much each has a native
oracle here. The plan is reread after S6.

## 7. Decisions for Tom

**D1. Where the services live.** (a) In `vapors-platform` beside the
window and input code. (b) A `vapors-services` crate the platform
depends on, so an app's non-UI code (touché's runtime, which compiles
without a window) can use the services without linking winit. touché's
runtime is exactly that case. **Recommend (b).**

**D2. A target without a backing.** (a) The implementation exists on
every target and answers `Unreachable(NotOnThisTarget(reason))`. (b)
The method is absent on that target and app code is `cfg`'d. (a)
keeps app code target-free and the refusal visible to the user; (b)
makes the limit a compile error, which is louder but puts `cfg` in
app code, which the lint forbids. **Recommend (a).**

**D3. Documents without indexes.** (a) Keys to versioned values under
a lock (3.4), as touché, Capacitor Preferences and Tauri `store`. (b)
IndexedDB's model (object stores, indexes, cursors, key ranges) on
both targets, with SQLite behind it natively. (b) is a database
service; no vapors app asks for one. **Recommend (a),** with (b) as a
later service if an app needs queries.

**D4. The native WebSocket client.** (a) `tokio-tungstenite` with
rustls: the standard client. (b) reqwest's websocket upgrade: one HTTP
client for both. (b) shares the TLS stack and deadlines already in
`Http`; its websocket support is newer. To be checked at S2's
successor against reqwest's version then. **Recommend (b) if it is
stable at the pinned reqwest, else (a).**

**D5. Linux file dialogs without a portal.** (a) The portal only, as
today: refused after 25 s where it is absent, which is this box. (b)
rfd's GTK backend as a fallback when the portal is absent and the app
is not sandboxed: a dialog on a plain desktop without a portal, at the
cost of linking GTK. (c) The portal, and the refusal made faster: a
`org.freedesktop.portal.Desktop` name check on the bus before the
call, so an absent portal answers `NoDialog` in milliseconds. (c) is
small and keeps one backend. **Recommend (c), then (b) only if a user
reports a portal-less desktop.**

**D6. WebRTC.** (a) Not a service: touché's realtime voice keeps its
page script for the peer connection, and the Rust side keeps the
decisions, as now. (b) A `Realtime` service wrapping
`RTCPeerConnection` on the web and `webrtc-rs` natively. (b) is
large and has one user. **Recommend (a)** until a second app or the
native touché app needs it.

**D7. Workers.** (a) `offload` only: a named function, owned data, a
`Cancel` token; no worker handle, no `postMessage`, no shared memory
in app code. (b) A `Worker` service exposing message passing. (a) is
design 01's decision; (b) would be a second way to reach a thread.
**Recommend (a).**

**D8. Notifications on macOS and Windows.** (a) Refused with
`NotOnThisTarget(NoBundle)` until vapors apps ship as bundles with
identifiers, stated in the docs. (b) Build the bundle machinery first
(an xtask that makes an `.app` and an MSIX), then the service. (b) is
factory work the positioning record lists under portability.
**Recommend (a) now,** and (b) when the platforms statement is
written.

**D9. Crypto's shape.** (a) Rust crates (`getrandom`, `sha2`, `ring`)
behind thin functions; no `SubtleCrypto`. (b) `SubtleCrypto` on the
web for non-extractable keys, with a native equivalent through the OS
key store. (b) matters only for a key that must never be readable by
the app, which no vapors app has. **Recommend (a).**

**D10. Placement.** (a) After the finish line, S1 to S6 first, since
touché carries their code today. (b) After the comparative benchmark
and the website. S1 to S6 remove code from touché and add nothing to
the perf workloads. **Recommend (a)**; Tom orders it against the
website's needs.

## 8. Risks

- **Two copies while moving.** Between S2 and the touché PR that
  switches to it, the transport exists in both repos; the pin bump
  (`.config/vapors-rev`) is the one commit that retires the copy, and
  the touché PR is part of each slice's acceptance.
- **Browser differences hide behind one API.** `showSaveFilePicker`,
  `navigator.share` with files, `clipboard.read` and `permissions.query`
  differ across Chromium, Firefox and Safari; each difference is an
  `Unreachable(NotOnThisTarget)` with the browser named, never a
  silent fallback except where the plan states one (save to download,
  3.5). Firefox and Safari have no automated oracle here (positioning
  record: not in any check).
- **Activation tokens change signatures.** Every picker, clipboard,
  share and notification call takes an `Activation`; touché's call
  sites move. The compile error is the migration list.
- **Deadlines from this box.** The storage deadlines were measured on
  this machine's Chromium; a phone's storage is slower, which the 80x
  margin covers (`profile_db.js`); a deadline that fires in the field
  is a measurement to redo, not a number to raise by hand.
- **macOS and Windows are untested** (positioning record). Every
  native row for them is written from the platforms' documentation
  and the crates' claims, not from a run here; the platforms statement
  and CI runners decide when they are proven.
- **Camera has no oracle here.** S14's native side is checked on a
  real device only; the PR says so and pins the web side.
- **`Absent` misuse.** The tempting conversion of a `None` or a `404`
  into `Absent` where the thing was in fact unreachable is the failure
  the three states exist to prevent; S1's `compile_fail` test on
  `From<Option<T>>` and review attention on every `Absent` arm are the
  guards.
