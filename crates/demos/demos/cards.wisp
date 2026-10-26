<script>
use crate::card::{Card, CardProps};

let tick: Signal<u32> = Signal::new(0);
let layers: Signal<Vec<u32>> = Signal::new((1..=20).collect());
</script>

<style>
/* Cards (1-R1): a scroll region of Card components, each a title row
   over children its parent passes: a thumbnail, a line of text that
   reads a signal of the parent's, and a button. */
:global(html), :global(body) { height: 100%; }
:global(html) { overflow: hidden; }
:global(body) { margin: 0; display: grid; grid-template-rows: 44px minmax(0, 1fr); background-color: #e9ecef; font-size: 14px; color: #171b1f; }
.bar { display: flex; align-items: center; padding: 0 12px; background-color: #171b1f; color: #e4e8ec; }
.stack { overflow: auto; scrollbar-color: #3a4148 transparent; scrollbar-width: thin; }
.thumb { width: 48px; }
.text { flex-grow: 1; }
button { padding: 6px 12px; border: none; border-radius: 6px; background-color: #3a7bd5; color: #ffffff; }
</style>

<div class="bar">Layers</div>
<div class="stack">{for layer in layers.get() key=layer}<Card title="Curves"><img class="thumb" src="sky.png"><div class="text">Layer {layer} at tick {tick}</div><button>Edit</button></Card>{/for}</div>
