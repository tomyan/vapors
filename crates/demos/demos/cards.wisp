<script>
use crate::card::{Card, CardProps};

let tick: Signal<u32> = Signal::new(0);
let layers: Signal<Vec<(i32, String, i32)>> = Signal::new(
    (1..=20_i32)
        .zip(["Curves", "Levels", "Saturation", "Warmth"].iter().cycle().skip(1))
        .map(|(id, kind)| (id, String::from(*kind), id * 5 - 50))
        .collect(),
);
</script>

<style>
/* The layer stack (1-R1): a keyed list of layers, each a Card titled
   with the layer's kind, its body the readout snippet declared once
   here and rendered per layer with the layer's values: a thumbnail, the
   amount, the parent's tick, and a button. */
:global(html), :global(body) { height: 100%; }
:global(html) { overflow: hidden; }
:global(body) { margin: 0; display: grid; grid-template-rows: 44px minmax(0, 1fr); background-color: #e9ecef; font-size: 14px; color: #171b1f; }
.bar { display: flex; align-items: center; padding: 0 12px; background-color: #171b1f; color: #e4e8ec; }
.stack { overflow: auto; scrollbar-color: #3a4148 transparent; scrollbar-width: thin; }
.thumb { width: 48px; }
.text { flex-grow: 1; }
button { padding: 6px 12px; border: none; border-radius: 6px; background-color: #3a7bd5; color: #ffffff; }
</style>

{snippet readout(kind: String, amount: i32)}<img class="thumb" src="sky.png"><div class="text">{kind} {amount} at tick {tick}</div><button>Edit</button>{/snippet}
<div class="bar">Layers</div>
<div class="stack">{for layer in layers.get() key=layer.0}<Card title={layer.get().1}>{render readout(layer.1.clone(), layer.2)}</Card>{/for}</div>
