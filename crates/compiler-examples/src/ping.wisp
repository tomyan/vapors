<script>
use crate::pong::{Pong, PongProps};
#[prop]
let depth: Signal<u32> = Signal::new(0);
#[dispatcher(ping)]
let events;
let tell = move |event: &mut Event| {
    if let Some(node) = event.target() {
        let _ = events.emit(node, Event::new("ping"));
    }
};
</script>

<div onclick={tell}>{if depth.get() < 2}<Pong depth={Signal::new(depth.get() + 1)} />{/if}</div>
