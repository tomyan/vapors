# Phones: vapors as a library inside Kotlin and Swift hosts

Date: 2026-11-19. Decided by: Tom. Plan: `docs/plans/platforms.md`
(3.5, 3.6, slices E1 to E4, A1 to A7, I1 to I6, D7). Nothing is built.

## Decision

1. **On Android and iOS vapors is a library inside a thin host** written
   in Kotlin or Swift, with starters and an integration skill. Vapors
   views should be embeddable in existing UIKit, SwiftUI, Android View
   and Compose apps.
2. **Desktop stays on winit.** `App` then has two drivers here (three
   with the web driver, `2026-11-18-web-driver.md`), meeting at one
   frame contract.
3. **The host hands over a surface** (`SurfaceView`, `CAMetalLayer`)
   through `raw-window-handle`, and forwards input, lifecycle and
   insets. Frames are on demand, matching `Next::Wait`.

## Why

- winit gives little on phones: no IME or soft keyboard, no pickers, no
  clipboard. A host in the platform's language uses `InputConnection`,
  `UITextInput` and the pickers directly instead of bridging each back
  to Rust.
- The winit route already needed a Gradle project with Kotlin or Java
  code and an Xcode project, plus JNI and objc2 bridges.
- `accesskit_android` and `accesskit_ios` attach to a host `View` or
  `UIView`.

## Costs

- An embedding API, with one FFI crate (uniffi or a C ABI) that carries
  the relaxed `unsafe` guard, declared where it runs.
- Two drivers of `App`.
- Several views in one app need more than one `App` and surface per
  process; `vapors-platform` drives one `App` in one window today. The
  plan's E3 settles whether that can be lifted.
- Claims to verify first are in section 10 of the plan: AccessKit with
  a host view, a wgpu surface from a host layer.
