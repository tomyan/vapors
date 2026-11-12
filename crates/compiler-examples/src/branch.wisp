<script>
use crate::branch::{Branch, BranchProps};
#[prop]
let depth: Signal<u32> = Signal::new(0);
#[dispatcher(fold)]
let events;
let tell = move |event: &mut Event| {
    if let Some(node) = event.target() {
        let _ = events.emit(node, Event::new("fold"));
    }
};
</script>

<div onclick={tell}>{if depth.get() < 2}<Branch depth={Signal::new(depth.get() + 1)} />{/if}</div>
