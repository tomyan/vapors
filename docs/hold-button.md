# HoldButton

`vapors_components::hold_button::HoldButton` exposes a `Signal<bool>` that is
true while at least one pointer or activation key is held. It is a compiled
`.wisp` component using the existing document event and capture APIs.

```html
<script>
use vapors_components::hold_button::{HoldButton, HoldButtonProps};
let held: Signal<bool> = Signal::new(false);
let enabled: Signal<bool> = Signal::new(true);
let reset: Signal<u64> = Signal::new(0);
</script>

<HoldButton label={String::from("Hold")}
    bind:pressed={held} enabled={enabled} reset={reset} />
```

Each pointer is captured on press. Release, cancellation or capture loss removes
only that pointer. Space and Enter are held independently; blur clears held
keys. Increment `reset` to clear all holds, for example when pausing an app.
Setting `enabled` false clears holds and blocks activation; enabling again does
not restore them. Unmount clears a surviving consumer's `pressed` signal.

The button uses `aria-disabled` and remains focusable when unavailable. This
checkout cannot dynamically remove HTML boolean attributes; the event guards
enforce availability. Native boolean-attribute support would allow a separate
choice to remove unavailable buttons from keyboard navigation.

Style instances using inherited `--hold-width`, `--hold-height`,
`--hold-background` and `--hold-color`. The racer supplies these from its
DrivingControls component; no game-specific input behavior lives in HoldButton.

The compiler currently reports that external component props are outside its
local validation pass. The generated Rust still type-checks the props struct.
