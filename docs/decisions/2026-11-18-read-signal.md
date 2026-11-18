# Read-only signals: `ReadSignal<T>` beside `Signal<T>`

Date: 2026-11-18. Decided by: Tom, with the lead.

## Decision

1. **`Signal<T>` stays the read-write handle** a script authors with:
   one name per piece of state, `Copy`, `get`/`set`/`update`. Solid's
   split getter and setter is not adopted, and the API does not push
   authors to take a reader first: that would add a step to every read.
2. **`ReadSignal<T>` is a read-only view**: `Copy`, with the reading half
   of `Signal`'s API (`get`, `with`, the `_untracked` and `try_` forms)
   and nothing that writes. `signal.read_only()` makes one, and
   `From<Signal<T>>` and `From<Memo<T>>` convert, so a read-only place
   accepts either.
3. **Computed values are read-only.** `Signal::derived` returns
   `ReadSignal<T>`, and so do `frame_time()` and `frame_delta()`. A
   write to a computed value would be overwritten at its inputs' next
   change; the type now refuses it at compile time instead.
4. **A prop says whether the child may write.** A prop typed
   `ReadSignal<T>` gives the child read access only; a prop typed
   `Signal<T>` is a deliberate grant of write access, as Svelte's
   `$bindable` is. The scene tags' `transform` and `projection` props
   take `ReadSignal`, so `transform={Signal::derived(..)}` and
   `transform={some_signal}` both work and neither lets the tag write.
5. **Later:** the compiler and `wisp-check` report which components can
   write a signal (the analysable-sources direction, and the one-writer
   rule in `docs/plans/3d-engine-parity.md` T2). Not in the first slice.

## Why

A single read-write handle carries write access wherever the value
goes, so the types say nothing about who writes; and `Signal::derived`
returned a writable signal whose writes were silently lost. A read-only
type fixes both without making authoring longer: most code keeps
writing `Signal<T>`, and handing out write access becomes a choice.
