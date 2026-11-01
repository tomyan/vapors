<script>
use crate::ping::{Ping, PingProps};
#[prop]
let depth: Signal<u32> = Signal::new(0);
#[dispatcher(pong)]
let events;
let tell = move |event: &mut Event| {
    if let Some(node) = event.target() {
        let _ = events.emit(node, Event::new("pong"));
    }
};
</script>

<div onclick={tell}>{if depth.get() < 2}<Ping depth={Signal::new(depth.get() + 1)} />{/if}</div>
