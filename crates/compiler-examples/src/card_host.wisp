<script>
use crate::card::{Card, CardProps};
use crate::frame::{Frame, FrameProps};

let count: Signal<i32> = Signal::new(0);
#[prop]
let outside: Signal<i32> = Signal::new(0);
</script>

<Card title="Curves"><p>count {count}</p>{if count.get() > 2}<em>many</em>{/if}<b>static</b><i>{outside}</i></Card>
<Frame><u>framed {count}</u></Frame>
