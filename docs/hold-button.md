# HoldButton

`vapors_components::hold_button::HoldButton` is held while at least one pointer
or activation key is down. It is a compiled `.wisp` component using the
existing document event and capture APIs.

The hold is output-only. The button keeps it in its own `held` memo and calls
its `on_change: Rc<dyn Fn(bool)>` prop once at mount and on each change.
`vapors_components::hold_into(signal)` builds an `on_change` that writes the
change into `signal`. The consumer holds no handle to the button's state, so
writing its own signal does not release the button.

```html
<script>
use vapors_components::hold_button::{HoldButton, HoldButtonProps};
let held: Signal<bool> = Signal::new(false);
let enabled: Signal<bool> = Signal::new(true);
let reset: Signal<u64> = Signal::new(0);
</script>

<HoldButton label={String::from("Hold")}
    on_change={vapors_components::hold_into(held)} enabled={enabled} reset={reset} />
```

Each pointer is captured on press. Release, cancellation or capture loss removes
only that pointer. Space and Enter are held independently; blur clears held
keys. Increment `reset` to clear all holds, for example when pausing an app.
Setting `enabled` false clears holds and blocks activation; enabling again does
not restore them. Unmount calls `on_change(false)` if the consumer was last told
`true`. When one owner's disposal takes both the consumer's signal and the
button, the signal is disposed before the button's cleanup runs; `hold_into`
lets that one release through and refuses any other failed write (the relaxed
guard is declared in `crates/components/src/hold_output.rs`).

The button uses `aria-disabled` and remains focusable when unavailable. This
checkout cannot dynamically remove HTML boolean attributes; the event guards
enforce availability. Native boolean-attribute support would allow a separate
choice to remove unavailable buttons from keyboard navigation.

Style instances using inherited `--hold-width`, `--hold-height`,
`--hold-background` and `--hold-color`. The racer supplies these from its
DrivingControls component; no game-specific input behavior lives in HoldButton.

The compiler currently reports that external component props are outside its
local validation pass. The generated Rust still type-checks the props struct.
