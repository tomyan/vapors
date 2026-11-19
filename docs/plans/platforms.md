# Platforms: vapors apps on web, Android, iOS, macOS, Windows and Linux

Status: design, 2026-11-12. For after the performance finish line
(`docs/plans/perf-finish-line.md`); nothing here changes current work.
Read from the code at 90d1c8a. Every claim about the code names its
file; every claim from outside the repository is listed in section 10
to be checked before the slice that relies on it starts.

Rules this design keeps: every failure loud (a platform that cannot do
something refuses by name, never approximates); runtime performance
first (no per-frame cost added for a platform that is not running);
one small shared runtime (the platform layer is one crate with
`cfg` modules, not a copy per target); no `unsafe` in vapors' own code
(`unsafe_code = "forbid"` in `Cargo.toml` `[workspace.lints.rust]`;
foreign calls live in narrowly scoped crates named in section 3, each
with the relaxed guard declared and its reason); accessibility is a
requirement on every target (`docs/design/05-accessibility.md`).

## 1. What exists today

**One window, one loop.** `vapors-platform` opens one winit window
(winit 0.30.13, wgpu 30.0.1: `Cargo.lock`) or attaches to the page's
canvas, and drives one `App` (`crates/platform/src/window.rs`). The
handler creates the window in `resumed` once and never handles
`suspended` (`crates/platform/src/handler_events.rs`); that is right on
desktops, where `suspended` does not come, and wrong on Android and
iOS, where the surface is destroyed on suspend (section 3). The phone
hosts would not use this handler at all (3.5).

**GPU.** Natively `wgpu::Backends::PRIMARY` (Vulkan, Metal, DX12), on
the web `BROWSER_WEBGPU` with no WebGL fallback
(`crates/platform/src/surface.rs`, touche `DESIGN.md` 1.4). The device
is opened at the defaults with `max_buffer_size`,
`max_texture_dimension_2d` and `max_storage_buffer_binding_size`
raised to the adapter's; an adapter below the default for any of the
three is refused by name (`crates/gpu/src/limits.rs`). The surface
format is the first 8-bit non-sRGB BGRA or RGBA (`surface.rs`
`pick_format`), which covers Metal's BGRA and Android's RGBA. A lost
device stops the loop and names the fix (reload, or restart the app);
there is no recovery (`crates/platform/src/device_lost.rs`). A surface
miss is waited out (`Occluded`, `Timeout`) or reconfigured, with 60
failures in a row stopping the loop (`surface_texture.rs`,
`hidden_window.rs`, `handler.rs` `MISSED_LIMIT`).

**Input.** Pointer, wheel, keys, characters, touch (winit `Touch` is
translated in `event_winit.rs:35`; gestures in `gestures.rs`), macOS
trackpad pinch (`input.rs:56`, winit's `PinchGesture`), paste
(`Event::Paste` natively, `Event::PastedText` on the web), window
focus (`event.rs`). The shortcut modifier is Command on macOS and on
Apple browsers, Control elsewhere (`shortcut_modifier.rs`).

**Text input and IME.** Natively winit's `Ime` events over the focused
`TextField`, composing text kept in the value as Chromium's `<input>`
holds it (`ime.rs`, `ime_window.rs`). On the web a hidden `<textarea>`
mirrors the field, so the phone keyboard opens and Android IMEs
compose (`text_bridge_web.rs`). `docs/design/QUESTIONS.md` Q9 decided
EditContext first (Tom, 2026-10-25); what is built is the textarea.
Section 8 asks which stands.

**Clipboard.** Native only, through arboard 3.7 (X11 and Wayland
data-control; macOS and Windows need no feature): the image with a
3 s deadline measured on this box, text on another thread
(`clipboard.rs`, `clipboard_text.rs`). The web reads only its `paste`
event's text; images reach touche through its own page script
(`~/projects/touche/crates/web/drop.js`, W1).

**Files.** Open: rfd 0.18 on a thread natively (the XDG portal on
Linux, native dialogs elsewhere; `picker.rs`), a hidden
`<input type=file>` on the web (`picker_web.rs`); the file arrives as
bytes (`Event::FileOpened`). No save picker; no drag-and-drop into the
window (winit's `DroppedFile` and `HoveredFile` are not translated in
`event_winit.rs`). touche's web page downloads or shares exports from
its own script (`docs/MVP.md` M4b).

**Menus.** None. `docs/design/07-platform-services.md` chose muda for
the native menu bar and an in-app menu on the web; neither is built.

**Accessibility.** The element tree projects an a11y tree
(`crates/elements/src/a11y_tree.rs`), takes only what changed
(`a11y_mirror.rs`), and the app's frame has an `accessibility` phase
judged by `vapors-perf --scaling` (R6 PRs 2 to 4: 9af8d24, 7e8c320,
7920929). The web mirror in the platform (R6 PR 5), focus and actions
(PR 6) and touche's adoption (PR 7) are not merged at 90d1c8a (no
`set_ax_update` in `crates/platform/src`). Native adapters are R7:
AccessKit through `accesskit_winit`, decided in `05-accessibility.md`;
no `accesskit` crate is in `Cargo.lock` yet.

**Build and CI.** Four jobs, all on `ubuntu-24.04`: `ci`, `web`
(trunk, headless Chrome), `gpu` (lavapipe, Chromium oracles),
`window` (Xvfb, lavapipe, xclip) (`.github/workflows/ci.yml`);
`cargo xtask gate` runs the same steps here. Tools are pinned in
`.config/tools.toml`. No other operating system builds anywhere.
winit's Linux backends are selected by feature (`x11`, `wayland`,
`wayland-csd-adwaita-notitle`: `Cargo.toml`); its macOS, Windows and
iOS backends compile by target with no feature, and Android needs
`android-native-activity` or `android-game-activity`, neither
enabled, so an Android build fails today at winit. The lock already
carries `objc2`, `jni`, `ndk` and `android-activity` through winit
(`Cargo.lock`), so the FFI crate the phone hosts need would add little
to the dependency tree, and the Android feature of winit is not needed
once phones leave winit (3.5).

**Threading.** The UI thread owns the graph, the tree, the scene and
queue submission (`docs/design/02-scheduler-frame-threading.md`,
"Threading"). macOS and iOS require the event loop on the main thread;
the model already assumes it.

## 2. What a real app needs, from touche

touche's decisions (`~/projects/touche/DESIGN.md`, `docs/MVP.md`):

- Desktop is the paid product; macOS (Metal) and Windows (DX12) are
  the targets, Linux is the development platform (`DESIGN.md` 2. Tech
  stack, "Platforms"). The web page is the free tier and the remote
  client (`DESIGN.md` 7. "Decided").
- Phones are the web page in Chrome; no native app for the MVP
  (`docs/MVP.md` 4.). Android and iOS native are "a later possibility,
  not a first-year goal" (`DESIGN.md` 7., "Decided: Android and iOS").
  The store listings can wrap the web build: a Trusted Web Activity on
  Play; on iOS a WKWebView wrapper is at risk under Apple's
  minimum-functionality rule, so the native build is the iOS path when
  scheduled (`DESIGN.md` 7., "Noted 2026-10-22"). Tom's assessment of
  the iOS native build is there too: medium-large, mostly glue, Android
  first because the Pixel is here. Tom decided on 2026-11-19 that the
  native phone builds are a library inside thin Kotlin and Swift hosts
  (3.5; `docs/decisions/2026-11-19-phones-as-library.md`).
- The app shell is a fixed viewport with safe-area insets as padding,
  the on-screen keyboard resizing the viewport, rotation keeping the
  zoom (`DESIGN.md` 3., "App shell").
- Installed web app: `manifest.webmanifest` (standalone, icons; M1b),
  the update notice from `version.json` written by trunk's post-build
  hook and compared with the running build's version, a Toast "Update
  ready, Reload" once no work would be lost (M7c: `crates/web/update.js`,
  `crates/web/Trunk.toml`, `design/patterns.md` "Update notice").
- Keys go in the OS keychain natively (`DESIGN.md` 1.8; `keyring`);
  exports download or open the share sheet (M4b); drop and paste open
  files on the desktop page (W1).
- The native app today is `touche open` (`crates/app/src/window.rs`):
  the viewer with a top strip, screenshotted under Xvfb by CI
  (`design/patterns.md` "Native top strip"). Desktop packaging is out
  of the MVP (`docs/MVP.md` 1.); signed releases are due at the first
  release (`FACTORY.md`, "Supply chain" row).
- Images over 16384 px on a side are refused
  (`crates/viewer/src/decode.rs:29`); the Pixel 9 Pro reports 16384
  under Chrome's WebGPU and this box's Intel UHD 630 and lavapipe
  report 16384 (`docs/gpu-limits.md`); headless Chromium on
  SwiftShader reports 8192, the default, which `raised_limits` allows.
  Phones with 6 to 16 GB need the proxy pipeline before large files
  (`DESIGN.md` 7.).

So a platform is done for touche when all of these hold on it: a
window with a GPU device and the limits reported; open, save, drop
and paste; keyboard with the platform's modifier; text input with the
platform's IME and soft keyboard; a screen reader pass (acceptance
step 6); a signed package with the update notice; suspend and resume
without a dead surface; and a CI job or a device check that proves
each on every pull request that touches it.

## 3. Per platform

Each platform below takes the same headings. "Built" means in the
code at 90d1c8a; "gap" means a slice in section 7.

### 3.1 Web

- Windowing and loop: winit's web backend on `<canvas id="vapors">`,
  the browser's loop, `run` returns at once (`window.rs`); the page
  contract in `docs/web.md`. Built.
- GPU: WebGPU only; Chrome or Edge 113+, Safari 26+, Firefox 141+,
  over https (`docs/web.md`). A browser without `navigator.gpu` reads
  the `NoWebGpu` error (`webgpu.rs`). Built. Render in a worker is a
  start-up option designed in `02-scheduler-frame-threading.md`, built
  only if timings show main-thread frame drops; not scheduled here.
- Text input and IME: the hidden textarea (`text_bridge_web.rs`),
  checked against Chromium's own `<input>` in CI
  (`scripts/web-text-input.sh`). Built. EditContext (Q9) is a decision
  (section 8, D2).
- Clipboard: paste text through the textarea's `paste` event. Gap:
  `navigator.clipboard` read and write with the activation rule
  (`07-platform-services.md` P2).
- Files: open through `<input type=file>` (`picker_web.rs`). Gaps: a
  save picker (File System Access API where present, else download);
  drop into vapors rather than the page's script.
- Menus: in-app, drawn by vapors (`07-platform-services.md`). Gap.
- Accessibility: the DOM mirror (`docs/plans/r6.md`), PRs 5 to 7 open.
  VoiceOver on iOS Safari is named unverified in r6.md until an iPhone
  runs it.
- Packaging and store: static files from `cargo xtask web`, served by
  `cargo xtask serve` or any static host; the PWA manifest is touche's.
  Play can list the installed page as a Trusted Web Activity
  (`DESIGN.md` 7.). Built on touche's side.
- Updates: `version.json` and the Toast (M7c). Built on touche's side.
  A vapors app that is not touche has nothing; a gap if a second app
  wants it (section 7, S5 shares the check with native).
- Lifecycle: a backgrounded tab's device loss stops the loop and says
  "reload the page" (`device_lost.rs`); `page_hidden` is read for
  deadlines (`timer_web.rs`). Built.
- Safe areas and touch: insets are the page's CSS `env()`
  (`DESIGN.md` "App shell"); vapors' `Environment` has no inset. Gap
  (S3). Touch and gestures built (`gestures.rs`); `touch-action: none`
  on the canvas is the page's job (`docs/web.md`).

### 3.2 Linux

- Windowing: winit X11 and Wayland, client-side decorations without a
  title on compositors that draw none (`Cargo.toml` winit comment,
  touche `DESIGN.md` platform notes). Built.
- GPU: Vulkan (ANV here, lavapipe in CI; `docs/gpu-limits.md`). Built.
- Text input and IME: winit's `Ime` (`ime.rs`), pinned by unit tests;
  a real IME (ibus, fcitx) has been exercised by no check. Gap: a
  by-hand checklist row, since no oracle drives ibus headless.
- Clipboard: arboard; checked under Xvfb with xclip in CI
  (`examples/check_clipboard`). Built.
- Files: rfd's XDG portal; a box without a portal gets `file dialog:
  <cause>` after libdbus's 25 s (`picker.rs`, `log_forward.rs`). Built.
  Gap: save picker; drag-and-drop (winit `DroppedFile`).
- Menus: muda (`07-platform-services.md`). Gap.
- Accessibility: AccessKit's AT-SPI adapter through `accesskit_winit`
  (`05-accessibility.md`); Orca is the sign-off reader. Gap (S6).
- Packaging and store: nothing. Options in D5: AppImage (one file,
  runs under Xvfb in CI), Flatpak (sandboxed; the portal picker already
  fits it; Flathub is the listing), `.deb`. No signing authority; a
  checksum and a signature beside the download.
- Updates: the `version.json` check natively with a download link
  (S5), or Flathub's own.
- Lifecycle: hidden-window waits (`hidden_window.rs`). Built.
- Safe areas and touch: none; winit `Touch` on touch screens, built.

### 3.3 Windows

- Windowing: winit's Win32 backend, compiled by target. Not built
  anywhere; the first slice is a `cargo check` for
  `x86_64-pc-windows-msvc` from Linux (checking needs no linker) and a
  CI job on a Windows runner (W1).
- GPU: DX12 and Vulkan are both in `PRIMARY`, so one physical GPU can
  enumerate twice; what `vapors_gpu::select` does with two adapters of
  one name on two backends (`crates/gpu/src/select.rs`) is checked by
  W1 and fixed if it picks by name alone. The runner's adapter is WARP
  (Microsoft's software DX12), the Windows counterpart of lavapipe;
  `VAPORS_GPU_ADAPTER` names it.
- Text input and IME: winit's `Ime` on Windows (TSF). The same
  `ime.rs` path; whether winit's Windows preedit matches the Linux
  sequence `ime.rs` assumes (an empty preedit before each commit) is a
  W1 check.
- Clipboard: arboard, no feature. Files: rfd's `IFileDialog`, no
  feature (`crates/platform/Cargo.toml`). Drag-and-drop: the same gap.
- Menus: muda's Win32 menu bar. Gap.
- Accessibility: AccessKit's UI Automation adapter; NVDA and Narrator
  by hand on a Windows machine, none here.
- Packaging and signing: an installer (NSIS or WiX) or MSIX from
  cargo-packager (D4). Authenticode signing: since 2023 the CA/B Forum
  requires OV code-signing keys in hardware or an HSM, so the practical
  routes are Azure Trusted Signing (a cloud service, monthly fee,
  identity validation) or an EV certificate on a token; an unsigned
  download gets a SmartScreen warning, and reputation under a new
  certificate builds with downloads (section 10 lists these to verify).
- Updates: the `version.json` check and a download link (S5); MSIX
  with App Installer updates itself, an option under D4.
- Lifecycle: `Occluded` waits; a driver reset (TDR) is a device loss,
  stop and restart (`device_lost.rs` `Target::Native`).
- Safe areas and touch: none; winit `Touch` on touch screens. winit
  documents `PinchGesture` for macOS and iOS; what a precision
  touchpad's pinch arrives as on Windows is a W1 check (section 10).

### 3.4 macOS

- Windowing: winit's AppKit backend, main thread, Retina scale factor
  in `Frame::scale_factor`. Not built anywhere; no Mac here.
- GPU: Metal; the surface offers BGRA, which `pick_format` takes.
- Text input and IME: winit's `Ime` through `NSTextInputClient`; the
  same `ime.rs` path and the same sequence check as Windows (M1).
  Command is the shortcut modifier (`shortcut_modifier.rs`), built.
- Clipboard: arboard on `NSPasteboard`. Files: rfd's `NSOpenPanel` and
  `NSSavePanel`. Drag-and-drop: the same gap.
- Menus: muda's `NSMenu`. On macOS the menu bar is not optional: Quit,
  About, Edit's Cut, Copy, Paste and the window menu are expected, and
  Cmd+Q goes through it. Gap (S7), ranked above the Windows and Linux
  bars for that reason.
- Accessibility: AccessKit's NSAccessibility adapter; VoiceOver by
  hand on a Mac.
- Packaging and signing: a `.app` bundle (`Info.plist`, icon) in a
  `.dmg`, from cargo-packager (D4); Developer ID signing with the
  hardened runtime and notarisation through `notarytool`, which needs
  the Apple Developer Program and a Mac with Xcode's command-line
  tools. The Mac App Store adds the sandbox and security-scoped
  bookmarks for file access; not scheduled (D6).
- Updates: the `version.json` check and a download link (S5). Sparkle
  is what native Mac apps use; it is an Objective-C framework and is
  not taken.
- Lifecycle: `Occluded` waits; no suspend.
- Safe areas and touch: none in the window; trackpad pinch is built
  (`input.rs:56`).

### 3.5 Android

**The approach for both phones.** On Android and iOS vapors would be a
library inside a thin host written in the platform's language (Kotlin,
Swift), not an app that owns the process through winit. The reasons:

- winit gives little on phones: no IME or soft keyboard on Android or
  iOS, no pickers, no clipboard. Every one of those is a platform API
  that a host in Kotlin or Swift calls directly (`InputConnection`,
  `UITextInput`, the pickers, `ClipboardManager`, `UIPasteboard`);
  with winit each would be bridged back into Rust through JNI or
  objc2.
- The winit plan already needed a Gradle project with Kotlin or Java
  activity code (A1, A5, D7) and an Xcode project (I1). The platform
  languages are in the build either way.
- `accesskit_android` and `accesskit_ios` attach to a host `View` or
  `UIView`; a host-owned view is where they would attach.

What it would cost, stated so the slices can be judged against it:

- Phones leave winit, so vapors needs an embedding API: the host hands
  a surface (a `SurfaceView` or `CAMetalLayer`) through
  `raw-window-handle` to wgpu, and forwards input, lifecycle and
  insets as calls. Frames would be on demand, matching `Next::Wait`:
  the host draws when the app asks and not on a timer of its own.
- Desktop stays on winit, so `App` has two drivers (three with the web
  driver, `docs/decisions/2026-11-18-web-driver.md`). Each translates
  its platform into the same events and frame contract; none would
  branch inside the app.
- Embedding vapors views in an existing app (UIKit, SwiftUI, Android
  Views, Compose) may need more than one `App` and surface in a
  process. `vapors-platform` drives one `App` in one window today
  (section 1); whether that can be lifted is E3's question, and until
  it is answered the claim "embeddable in existing apps" is only
  proven for one view per process.
- The FFI boundary is one crate (uniffi or a C ABI; E1 picks) with the
  relaxed `unsafe` guard declared in its `Cargo.toml` and the reason
  in its `lib.rs`, where the boundary runs. App code never sees it.

**Android.**

- Entry and windowing: the host is a Kotlin `Activity` (or a Compose
  or View `ViewGroup` for an embedded view) holding a `SurfaceView`.
  Its `SurfaceHolder.Callback` would call the library on surface
  created, changed and destroyed; the library creates the wgpu surface
  from the `ANativeWindow` and drops it on destroy (S2's table). No
  `android_main`, no winit activity feature, no `GameActivity`.
- GPU: Vulkan only (`PRIMARY` has no GLES), so a device without a
  Vulkan driver is refused by name, consistent with "no fallback". The
  Pixel 9 Pro's Mali (Valhall) reported 16384 and f16 under Chrome's
  WebGPU (`~/.claude/.../tailscale-web-serving.md`, 2026-10-22); what
  wgpu's Vulkan backend reports natively is A2's first line. A
  known-bad driver list is `DESIGN.md`'s condition; it starts empty and
  grows from refusals seen on devices, never from guesses.
- Text input and IME: the host view would implement
  `onCreateInputConnection` and an `InputConnection` for the focused
  field, showing and hiding the soft keyboard through
  `InputMethodManager`. Composition and selection go to the library as
  `Event::TextEdited`, the same events `text_bridge_web.rs` produces
  (A4).
- Clipboard: `ClipboardManager` called from the host; the text or image
  reaches the library as the answer to a clipboard request, with the
  three states and the deadline of `07-platform-services.md`.
- Files: the Photo Picker, `ACTION_OPEN_DOCUMENT` and
  `ACTION_CREATE_DOCUMENT` or the share sheet run in the host's
  activity with the standard result callbacks; the bytes reach the
  library as `Event::FileOpened` (A5).
- Menus: in-app.
- Accessibility: `accesskit_android` attached to the host `View`, fed
  `AxUpdate` through the FFI crate (`05-accessibility.md` names the
  crate available as of 2026-10-25; that it works with a host view is
  a claim in section 10, checked in A6). TalkBack on the Pixel by
  hand; `adb shell uiautomator dump` reads the tree for a scripted
  check.
- Packaging and store: an App Bundle (`.aab`), Play App Signing, the
  64-bit requirement, a target API level Play moves up yearly; the
  host's Gradle project owns all of it. The Trusted Web Activity
  listing of the web page is the alternative with no native build
  (D3).
- Updates: Play. A sideloaded APK during development uses S5's check
  with no download.
- Lifecycle: the host forwards `onPause`, `onResume`, surface
  destruction and `onTrimMemory`; low-memory kills make the proxy
  pipeline the condition for large files (`DESIGN.md` 7.); rotation is
  a size and a scale call, each asking for a frame.
- Safe areas: the host reads `WindowInsets` (display cutout, system
  bars, soft keyboard) and passes them in; the library puts them in
  `Environment` as `env(safe-area-inset-*)` (S3). The host handles the
  keyboard resize through `windowSoftInputMode="adjustResize"` or the
  insets, the native counterpart of `interactive-widget=resizes-content`.
- Touch: the host forwards `MotionEvent`s as pointer and touch events
  to the library, into `gestures.rs`, the same recogniser the web
  uses.

### 3.6 iOS

The approach and its costs are in 3.5.

- Entry and windowing: the host is a Swift app: a `UIViewController`
  with a `UIView` whose layer is a `CAMetalLayer`, or a
  `UIViewRepresentable` for SwiftUI. The host owns the main thread and
  the run loop (the model assumes the UI thread), the signing, the
  icon set and the launch screen. The library creates the wgpu surface
  from the layer. Nothing here can build: no Mac.
- GPU: Metal. Apple kills an app that renders while backgrounded and
  jettisons one that grows past a few GB (`DESIGN.md` 7., "two to
  three gigabytes"), so S2's suspend path and the proxy pipeline are
  conditions. The host would stop drawing on
  `UIApplication.didEnterBackgroundNotification` and tell the library.
- Text input and IME: the host view adopts `UIKeyInput` or
  `UITextInput` for the focused field and shows the keyboard with
  `becomeFirstResponder`; marked text and selection go to the library
  as `Event::TextEdited` (I3).
- Clipboard: `UIPasteboard` called from the host. Files:
  `PHPickerViewController` and `UIDocumentPickerViewController`, the
  share sheet `UIActivityViewController`, presented by the host; the
  result reaches the library as an event (I4).
- Menus: in-app; `UIMenu` for iPad keyboard shortcuts is optional.
- Accessibility: `accesskit_ios` attached to the host `UIView`
  (`05-accessibility.md` names it available, correcting touche's note;
  the largest unknown per `DESIGN.md`; that it works with a host view
  is in section 10, checked in I5 by VoiceOver on a device).
- Packaging and store: Apple Developer Program, signing, TestFlight,
  review. A native app is not a thin wrapper, so the
  minimum-functionality rule does not apply; the paid tier unlocked by
  a subscription bought elsewhere is `DESIGN.md`'s reading of the
  store rules, re-read against the current guidelines before I6.
- Updates: the App Store.
- Lifecycle: the host forwards scene and application state changes,
  memory warnings and rotation.
- Safe areas: the host reads the view's `safeAreaInsets` and passes
  them in (S3); winit's support no longer matters.
- Touch: the host forwards `UITouch` events, and `UIPinchGestureRecognizer`
  where it is used, to the recogniser in `gestures.rs`.

## 4. What can be tested where

| Target | On this box (headless Debian, Intel UHD 630, lavapipe, Xvfb, Chromium 155) | Needs a device or another machine |
|---|---|---|
| Web | Headless Chromium on SwiftShader: the page contract, text input, a11y mirror oracle, wake frames (`ci.yml` web and gpu jobs); the Pixel 9 Pro over the tailnet (`cargo xtask serve`) | Safari and Firefox: a Mac, an iPhone; WebGPU in each is listed in `docs/web.md` |
| Linux | Everything in the `window` job: Xvfb, lavapipe, xclip; Wayland under `weston --backend=headless` is possible and not in CI | A desktop session for Orca, the portal picker, a real IME |
| Windows | `cargo check --target x86_64-pc-windows-msvc` (no linker needed); nothing runs | A GitHub `windows-latest` runner: build, unit tier, gpu tier on WARP, a window on the runner's desktop session; NVDA by hand on a Windows machine |
| macOS | Nothing: no Apple SDK on Linux | The Intel Mac over ssh when available: build, unit and gpu tiers on Metal, signing and notarisation (it needs a macOS Apple still supports with Xcode; see section 10 on the last Intel release); a GitHub macOS runner (Apple silicon) for CI; the Mac Studio for VoiceOver, windows and the trackpad |
| Android | `cargo ndk` build for `aarch64-linux-android` with the NDK installed; the Pixel 9 Pro over `adb` (USB or wireless debugging): install, run, `logcat`, `screencap`, `uiautomator dump`; the emulator on this box has no GPU and its Vulkan is unverified | Firebase Test Lab or another device cloud for other GPUs; TalkBack by hand on the Pixel |
| iOS | Nothing | A Mac with Xcode: the simulator (Metal works there), a device for VoiceOver, touch and memory; GitHub macOS runners for the build and simulator tests |

CI cost: GitHub's macOS minutes are priced about ten times Linux's and
Windows about twice; a macOS job runs on pull requests that touch the
platform crate or the app, not on every push (the gate's job `if` rules
in `gate_plan_job_if_tests.rs` already express conditions).

## 5. Existing practice, with the real differences

| Project | What it does | Same as vapors | Different | Taken |
|---|---|---|---|---|
| Tauri | A Rust process around the OS webview; `tauri-bundler` makes `.app`, `.dmg`, `.msi`, NSIS, AppImage, `.deb`; a signed updater plugin; mobile through generated Xcode and Gradle projects | Rust core, per-platform shells generated | The UI is the platform's webview, not a GPU renderer; vapors draws itself and has no JavaScript side natively | cargo-packager (the bundler extracted from Tauri as a crate), pinned in `tools.toml` (D4); the generated Xcode and Gradle shells as the shape of our starters (E4); not its process-owning mobile entry, since vapors' phone hosts would own the process and call the library |
| Slint | Own language compiled to Rust, own renderers (femtovg, Skia, software), winit, AccessKit on desktops, Android through its activity crate, iOS experimental | winit plus a GPU renderer plus AccessKit, one tree projected to a11y | A software renderer fallback where vapors refuses without a GPU; its own language where vapors takes HTML and CSS | Nothing for phones: its winit-based Android entry is the route vapors would not take; the activity crate remains a reference for what a Java side must do |
| Flutter | Dart, the Impeller renderer, an embedder per platform written in the platform's language (Objective-C, Kotlin), semantics tree pushed to each platform, a hidden DOM for semantics on the web | The hidden-DOM a11y mirror on the web (`05-accessibility.md` names it); one renderer on every target | Platform code in native languages with a message channel; vapors' hosts would be Kotlin and Swift calling one Rust FFI crate with typed calls and events, no channel | The embedder shape: a host in Kotlin and Swift owning input, text, pickers and the surface, the engine behind a narrow API; and its add-to-app route for views inside existing apps, the reference for E3 (from memory; section 10) |
| Bevy | winit and wgpu on every target including Android (GameActivity by default) and iOS (an Xcode project in its mobile example), AccessKit on desktops | The same window and GPU crates, so the same suspend and resume shape | No IME on phones, no a11y on phones, no packaging tooling | The rule that the surface is dropped on suspend and recreated on resume, which S2 and the host callbacks would apply without winit |
| Dioxus and Blitz | Dioxus renders through the webview (wry) or through Blitz: Stylo for CSS, Taffy for layout, Parley for text, Vello for paint on wgpu; `dx` builds mobile projects | Blitz is the closest sibling: HTML and CSS to a GPU through Taffy and Parley, the same two crates vapors uses (`Cargo.toml`) | Stylo and Vello where vapors has its own CSS engine and painter; a virtual DOM where vapors compiles components | Nothing to take yet; a comparison for the benchmark the finish line defers |
| egui | winit and wgpu, AccessKit through `accesskit_winit`, Android through `eframe`, no official iOS | The `accesskit_winit` integration, activated on the reader's first query so an app with no reader pays nothing (`05-accessibility.md` "Taken from where") | Immediate mode; no text IME on phones | The adapter lifecycle on desktops; phones would not use `accesskit_winit` |

What none of them give: a platform layer (one crate on desktops, one
FFI crate on phones) whose every answer
is three states with a deadline (`07-platform-services.md`); each has
a plugin or channel surface vapors rules out.

## 6. Order, by value to touche's customers

Retouchers are on macOS and Windows (`DESIGN.md` 2.); the paid tier is
the desktop; the web page is the free tier and is shipping. So:

1. **Web**: keep it whole. R6 PRs 5 to 7 finish the a11y mirror; the
   clipboard and save services (S-web) close the page's own gaps.
2. **The shared slices** (S1 to S7): the lifecycle, insets, packaging
   command, update check, native a11y, drop and save, the menu bar.
   Each is proven on Linux here, under Xvfb, and is what every other
   desktop needs before it can ship.
3. **Windows**: testable on a GitHub runner today and half of the
   desktop market for the customers; signing is the long pole and can
   start now (D4, D5).
4. **macOS**: the other half of the customers and the one Tom plans
   hardware for. Build and sign on the Intel Mac over ssh as soon as it
   is reachable; VoiceOver and the trackpad when the Mac Studio comes.
   If the Intel Mac arrives before the Windows signing route is
   settled, macOS goes before Windows: the order between the two is a
   test-capability call, not a value call (D1).
5. **Linux**: ships as the dev platform's by-product; an AppImage from
   the packaging command costs one slice and is the CI proof of the
   command.
6. **Android native** (a Kotlin host around the library, 3.5): the
   Pixel is here and a11y has an adapter, so it is the first phone; it waits for the proxy pipeline and for the
   desktops, since the Play listing can be the TWA meanwhile (D3).
7. **iOS native** (a Swift host, 3.5): after Android, when a Mac and an iPhone exist;
   Safari's home-screen install covers it until then (`DESIGN.md` 7.).

## 7. Slices

Every slice is one pull request under about 400 lines, tests first,
each check proven by breaking what it guards. "Accepted when" is what
the review and the gate hold it to. Sizes are estimates for order, not
commitments.

### Shared

| # | Delivers | Accepted when |
|---|---|---|
| S1 | `cargo xtask targets`: `cargo check -p vapors-platform -p vapors-app` for `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `aarch64-linux-android` and `aarch64-apple-ios` with the targets from `rust-toolchain.toml`; refuses naming the missing target or `cargo ndk` | A Linux-only `use` outside `cfg` in the platform crate fails the step naming the target; the step is in `ci.yml` and the gate; a missing target is a refusal, never a skip |
| S2 | Suspend and resume: `suspended` (winit) or the host's surface callbacks drop the surface and the window where the platform destroys them, `resumed` or the host's callbacks recreate them, the app gets `Event::Suspended` and `Event::Resumed`; the decision table is a pure function as `hidden_window.rs` is | Unit tests on the table (suspend during a frame, resume before the GPU was ready, two suspends); `surface_gradient --check` under Xvfb unchanged; the fake driver proves a frame after suspend without resume is refused by name |
| S3 | Safe-area insets in `Environment` as `env(safe-area-inset-*)`, fed by the platform: 0 on desktops, the page's computed `env()` on the web (read once per resize from a probe element), the host's insets on phones; `Frame` carries them | A demo pads by the insets; the page oracle sets `env()` through a viewport-fit page and reads the same box; an inset that is not finite is refused naming the side |
| S4 | `cargo xtask bundle <package>`: cargo-packager pinned in `tools.toml`, producing the AppImage on Linux, `.app` and `.dmg` on macOS, NSIS on Windows, from one `[bundle]` table in `factory.toml` (name, identifier, icon); refuses naming the missing tool, icon or identifier | The Linux AppImage runs under Xvfb in CI and prints its version line; a bundle with a missing icon is refused naming the path; `cargo xtask bundle --help` lists the targets |
| S5 | The update check natively: the same `version.json` as touche's page, fetched with the page's 10 s deadline (`update.js`), compared with the build's stamp, surfaced as a platform event the app shows as a Toast with a link; no download, no self-replacement | A stand-in server serving another version produces the event within the deadline; the same version produces nothing; an unreachable server is `Unreachable`, never "up to date"; the check and its numbers are shared with the page by a test that reads both |
| S6 | Native a11y (R7): `accesskit_winit` taking `AxUpdate` (`a11y_mirror.rs`) as the DOM mirror does, the same records and placements, actions back through dispatch; activated on the reader's first query | A fake adapter receives the same tree the model mirror holds for every R6 unit scenario; a `Click` action on a button runs the demo's handler; AT-SPI under `dbus-run-session` with `at-spi2-core` under Xvfb exposes the counter's button by name, read back with `pyatspi` or `accerciser`'s CLI in CI; Orca by hand on a session |
| S7 | Drop, save and the menu bar: winit `DroppedFile` and `HoveredFile` as `Event::FileDropped` and a hover state; `Platform::save_file` through rfd's save dialog and the web's download or File System Access; muda's menu bar with the platform's standard items (macOS app menu, Edit, Window) and the app's commands as the same shortcut table the web uses | Unit tests on the translations; a saved file round-trips under Xvfb; a `cargo xtask screenshot` of a window with the bar on Linux; macOS and Windows bars checked in M1 and W1 |

### Web (closing the page's own gaps)

| # | Delivers | Accepted when |
|---|---|---|
| Web1 | R6 PRs 5 to 7 as planned in `docs/plans/r6.md` | r6.md's own acceptance |
| Web2 | Clipboard service (P2 of `07-platform-services.md`): read and write text and images, activation rule, three states | The headless Chromium round trip with `Browser.grantPermissions`; a read outside activation refused before asking the browser |
| Web3 | Save: File System Access where present, download elsewhere, as one `save_file` (S7's web half) | Chromium's file chooser interception in the page contract |

### Windows

| # | Delivers | Accepted when |
|---|---|---|
| W1 | A `windows` CI job: build, unit tier, gpu tier with `VAPORS_GPU_ADAPTER` naming WARP, `clear_window` and the hello demo screenshot on the runner's desktop; the adapter-twice check in `select.rs`; the IME sequence and touchpad pinch findings recorded in this document | The job is green and gated; a GPU enumerated under two backends is picked once, by a test with two fake adapters |
| W2 | The NSIS installer from S4 on the runner; Start menu entry, file associations for JPEG, PNG and TIFF | The installer runs silently on the runner and the installed binary prints its version |
| W3 | Signing through the route D5 picks, from the runner with secrets; SmartScreen's first download checked by hand | A signed binary passes `signtool verify`; an unsigned release build is refused by the release step naming the fix |
| W4 | NVDA pass on a Windows machine by hand, with the checklist from `05-accessibility.md` "Screen readers"; findings as slices | The checklist filled in and committed |

### macOS

| # | Delivers | Accepted when |
|---|---|---|
| M1 | A `macos` CI job (Apple silicon runner) on pull requests touching the platform or app crates: build, unit tier, gpu tier on Metal, the clear-window screenshot; the IME sequence finding recorded | Green and gated on the matching PRs |
| M2 | The `.app` and `.dmg` from S4; the menu bar's macOS items from S7 checked on the runner | The bundle launches on the runner and prints its version |
| M3 | Developer ID signing and notarisation in the release step, from the Intel Mac over ssh or the runner, secrets by name; refuses when any is missing | `spctl --assess` passes on the notarised bundle; `stapler validate` passes |
| M4 | VoiceOver pass by hand when the Mac Studio comes; findings as slices | The checklist committed |

### Phones, shared

| # | Delivers | Accepted when |
|---|---|---|
| E1 | The embedding API and its FFI crate, `vapors-embed`: the relaxed `unsafe` guard declared in its `Cargo.toml` with the reason in `lib.rs`; the choice between uniffi and a C ABI recorded with the reason; calls to create from a `raw-window-handle` surface, resize and scale, pointer and key input, suspend and resume, insets, and "draw a frame"; events out (frame wanted, text edited, file opened) | A Rust test host drives an `App` through the API on lavapipe with a fake window handle: a frame only when the app asked (`Next::Wait` draws 0 frames idle), a frame after suspend without resume refused by name, a non-finite inset refused naming the side; the guard is checked by a test that reads the manifest |
| E2 | The winit driver and the embedding driver share the frame contract: one function from events to `Frame` used by both, so `App` has two drivers and no branch inside it | The existing `window` job unchanged and green; one test feeds the same event script to both drivers and compares the frames |
| E3 | The multi-view question: whether one process can hold several `App`s and surfaces (device and queue shared or one per view), for views embedded in an existing app. First a written finding from reading `crates/platform` and `crates/gpu` for what is global; then a test with two `App`s on two fake surfaces | The finding is committed with the file and line of each global; if two views work, the test shows both drawing and idle at 0 frames; if not, the API refuses a second view by name and the plan says one view per process |
| E4 | Starters and the integration skill: a Kotlin starter (Gradle project) and a Swift starter (Xcode project) under `platforms/`, each a host around `vapors-embed`, and a skill that tells an agent how to add vapors to an existing Android or iOS app. The skill lives in `plugins/` and the lead updates it when the pin moves | The Android starter builds here with the NDK (A1); the Swift starter builds on a Mac (I1); each starter's README lists the host calls it makes, and a test compares that list with the API's functions so a new call without a README line fails |

### Android

| # | Delivers | Accepted when |
|---|---|---|
| A1 | The Kotlin host in `platforms/android/`: a `SurfaceView` whose `SurfaceHolder.Callback` calls `vapors-embed`, a Gradle project linking the library through `cargo ndk`; `cargo xtask android build`, refusing when the NDK, SDK or Gradle is missing | The APK builds here; the refusals name each missing tool |
| A2 | `cargo xtask android run`: install and start on the attached Pixel, `logcat` filtered to the app's prefix, `screencap` to a file; the adapter line (name, limits) recorded in `docs/gpu-limits.md` by a `gpu-info` run on the device | The hello demo's screenshot from the Pixel is held to `hello.pixels` at the device's scale, or the difference is named |
| A3 | S2 on the device through the host's lifecycle calls: background and foreground ten times, rotation, the soft keyboard | A scripted `adb shell input keyevent` sequence leaves the app drawing and `logcat` shows no device loss |
| A4 | The `InputConnection` in the host view: `Event::TextEdited` from composition, the soft keyboard shown for a focused field, hidden when none | The web text-input scenario replayed on the device gives the same field text and selection after each step |
| A5 | Pickers, clipboard and share in the host: Photo Picker and `ACTION_OPEN_DOCUMENT`, `Event::FileOpened` as on the web; export through `ACTION_CREATE_DOCUMENT` or the share sheet | A file from the camera roll opens; a cancel sends nothing; an activity that is gone is `Unreachable` |
| A6 | `accesskit_android` on the host view taking `AxUpdate`; TalkBack on the Pixel | `uiautomator dump` shows the counter's button by name; a TalkBack double-tap changes the count once (the R6 PR 6 check) |
| A7 | The App Bundle, Play App Signing, the target API level, the listing's data-safety answers derived from `DESIGN.md` 1.6 | An internal-testing upload accepted by Play |

### iOS

| # | Delivers | Accepted when |
|---|---|---|
| I1 | The Swift host in `platforms/ios/`: a `UIView` on a `CAMetalLayer` calling `vapors-embed`, the Xcode project linking the static library, `cargo xtask ios build` on a Mac, refusing without Xcode | The hello demo on the simulator, screenshot held to its pixels |
| I2 | S2 and S3 on a device through the host's calls: suspend, resume, rotation, safe areas | Scripted through `xcrun simctl` on the simulator; a device by hand |
| I3 | The `UITextInput` conformance in the host view | The web text-input scenario on the simulator |
| I4 | Pickers, clipboard and the share sheet in the host | A photo opens; a cancel sends nothing |
| I5 | `accesskit_ios` on the host `UIView` taking `AxUpdate`; VoiceOver on a device | The counter by name in the Accessibility Inspector; one press per double-tap |
| I6 | Signing, TestFlight, the review answers | A TestFlight build installed on Tom's iPhone |

## 8. Decisions for Tom

| # | Question | Options | Recommendation |
|---|---|---|---|
| D1 | Which desktop after the shared slices | (a) Windows first: a runner tests it today. (b) macOS first: the customers, once the Intel Mac is reachable | (a) unless the Intel Mac is reachable before W1 starts; then (b). The shared slices come first either way |
| D2 | Web IME: Q9 decided EditContext first; the textarea is built and checked in CI | (a) Keep the textarea, retire Q9's choice in `QUESTIONS.md` with the date. (b) Build EditContext as a second path with the status line saying which runs | (a): the textarea passes the CI oracle against Chromium's `<input>` on every push; EditContext is a second path with no failing case to justify it. Reopen if the a11y mirror's focused-field piece (r6.md) needs it |
| D3 | Phones on the store | (a) The TWA listing of the web page now, native Android after the desktops. (b) Native Android before the desktops | (a): `DESIGN.md` says phones are the web page and desktop is the product; the TWA costs no vapors work |
| D4 | Packaging tool | (a) cargo-packager, pinned, one `[bundle]` table. (b) Own scripts per platform | (a): one tool covers AppImage, `.app`, `.dmg`, NSIS and MSI; xtask refuses when it is missing and verifies its output by running it, so its failures stay loud |
| D5 | Windows signing | (a) Azure Trusted Signing (cloud, monthly, identity validation). (b) An EV certificate on a hardware token | (a) if Tom's business passes its validation; it signs from a runner with no token to ship. Check eligibility first (section 10) |
| D6 | macOS distribution | (a) Developer ID with notarisation, download from the site. (b) The Mac App Store as well | (a): `DESIGN.md` sells the desktop on the site; the store's sandbox adds file-access work for no customer reached |
| D7 | Phone architecture (decided by Tom, 2026-11-19) | (a) vapors as a library in thin Kotlin and Swift hosts, with starters and a skill. (b) winit's Android and iOS backends with GameActivity or NativeActivity and objc2 bridges | (a), decided: the platform's own APIs do text input, pickers and clipboard, and the accessibility adapters attach to a host view. Costs are the embedding API, two drivers of `App`, and the multi-view question (3.5, E1 to E4) |
| D8 | Linux package | (a) AppImage first, Flatpak when asked. (b) Flatpak first | (a): the AppImage is the CI proof of S4 under Xvfb; Flatpak needs a runtime manifest and a build step outside cargo |
| D9 | Updates natively | (a) The notice and a download link, the page's mechanism. (b) In-place self-update | (a) first; (b) only with a signed manifest, after D5 and M3 exist, since replacing a binary without a signature check is a supply-chain hole |

## 9. Risks

- **AccessKit on phones.** `05-accessibility.md` names `accesskit_android`
  and `accesskit_ios` as available on 2026-10-25; their fit with
  a host `View` or `UIView` and their coverage of text are unknown here.
  A6 and I5 are the checks; if either adapter cannot take the tree, the
  phone slips behind a web-mirror path in a WebView, which is the TWA.
- **The embedding API.** It is new surface that every phone host
  depends on. If it is wide, the hosts drift from it; E4's README check
  and E1's tests are what hold it. The one FFI crate carries the
  relaxed `unsafe` guard; it stays small by the 200-line rule, one file
  per responsibility.
- **One view per process.** If E3 finds global state that cannot be
  lifted, embedding in an existing app works for one vapors view at
  a time, and the plan must say so to those who ask.
- **Two drivers.** A behaviour fixed in the winit driver and not in
  the embedding driver would diverge; E2's shared function and the
  same-script test are the guard.
- **Android GPU drivers.** Vulkan-only with no fallback refuses some
  devices by name. The refusal is the design; the risk is the number.
  A device cloud run (A2's follow-up) measures it before Play.
- **iOS memory and background rendering.** Without the proxy pipeline
  a 24 MP source and its caches exceed what iOS allows; the pipeline is
  a condition, not a risk to accept.
- **Signing logistics.** Trusted Signing's validation, Apple's
  program, and SmartScreen reputation all take calendar time outside
  the factory. D5 and D6 start the clocks early.
- **CI cost and flakiness.** macOS minutes and WARP's speed; the
  `if` rules keep macOS to the PRs that need it, and every deadline is
  measured on the runner before it is set (`ci.yml`'s comments are the
  precedent).
- **A second source of truth.** touche's page scripts (drop, update,
  share) do what vapors' services will do; until each service lands,
  two places answer the same question. S5, S7 and Web2 retire them one
  at a time, each with a test that the page uses the service.
- **Backend duplicates on Windows.** One GPU twice in `select`; W1
  checks before anything else runs there.

## 10. Claims to verify before the slice that relies on them

From memory, not from the repository; each is checked at the start of
the slice named, and this section is updated with the date and result.

| Claim | Checked by |
|---|---|
| winit 0.30 compiles its macOS and Windows backends with no feature; the platform crate builds for the phone targets without winit's phone backends | S1 |
| `cargo check --target x86_64-pc-windows-msvc` needs no linker on Linux once the target's `rust-std` is installed | S1 |
| cargo-packager produces AppImage, `.app`, `.dmg`, NSIS and MSI from one configuration and is maintained | S4 |
| GitHub's Windows runners expose WARP to wgpu's DX12 backend and have a desktop session a winit window can open in | W1 |
| GitHub's macOS runners run wgpu's Metal tests (wgpu's own CI does) and can open a winit window | M1 |
| Apple's macOS 26 is the last release for Intel Macs (WWDC 2025), so the Intel Mac over ssh signs and notarises while 26 is supported | M3 |
| Azure Trusted Signing accepts an individual or a company with Tom's business history; the CA/B Forum hardware-key rule for OV certificates since June 2023 | D5, W3 |
| winit's `PinchGesture` on Windows precision touchpads; winit's Windows and macOS IME preedit sequences match `ime.rs`'s assumption of an empty preedit before a commit | W1, M1 |
| Play's current target API level and the App Bundle requirement | A7 |
| `accesskit_android` and `accesskit_ios` exist as published crates | A6, I5 |
| The App Store guideline on features unlocked by a purchase made elsewhere, as `DESIGN.md` reads it | I6 |
| The Pixel 9 Pro's limits under wgpu's Vulkan backend equal those Chrome's WebGPU reported (16384, f16) | A2 |
| `accesskit_android` and `accesskit_ios` can be attached to a host `View` or `UIView` that the host app owns, and take their tree from calls through an FFI crate rather than from a winit window | A6, I5 |
| A wgpu surface can be created from a host's `ANativeWindow` (from a `SurfaceView`) and from a `CAMetalLayer` through `raw-window-handle`, and be dropped and recreated on the host's surface callbacks | E1, A1, I1 |
| An Android `InputConnection` and an iOS `UITextInput` on a view that vapors draws into carry composition and selection to the library as the web textarea does | A4, I3 |
| uniffi (or a C ABI) can carry the embedding API's calls and events with the relaxed guard in one crate | E1 |
| Flutter's add-to-app embeds a view in an existing Android or iOS app, and how it holds several engines | E3 |
