<script>
use crate::picker::{Picker, PickerProps};

let shown: Signal<bool> = Signal::new(false);
let heard: Signal<u32> = Signal::new(0);
let on_pick = move || heard.update(|n| *n += 1);
</script>

<div onpick={on_pick}>{if shown.get()}<Picker />{/if}</div>
