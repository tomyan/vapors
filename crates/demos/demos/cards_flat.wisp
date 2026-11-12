<script>
let tick: Signal<u32> = Signal::new(0);
let layers: Signal<Vec<(i32, String, i32)>> = Signal::new(
    (1..=20_i32)
        .zip(["Curves", "Levels", "Saturation", "Warmth"].iter().cycle().skip(1))
        .map(|(id, kind)| (id, String::from(*kind), id * 5 - 50))
        .collect(),
);
</script>

<style>
/* The cards demo's tree built without snippets, for snippet_cost: the
   same elements and styles, the card markup written out in place. */
:global(html), :global(body) { height: 100%; }
:global(html) { overflow: hidden; }
:global(body) { margin: 0; display: grid; grid-template-rows: 44px minmax(0, 1fr); background-color: #e9ecef; font-size: 14px; color: #171b1f; }
.bar { display: flex; align-items: center; padding: 0 12px; background-color: #171b1f; color: #e4e8ec; }
.stack { overflow: auto; scrollbar-color: #3a4148 transparent; scrollbar-width: thin; }
section { margin: 8px; background-color: #ffffff; border: 1px solid #c7ccd1; border-radius: 6px; }
h2 { margin: 0; padding: 8px 12px; font-size: 14px; background-color: #171b1f; color: #e4e8ec; border-radius: 5px 5px 0 0; }
.body { display: flex; align-items: center; gap: 12px; padding: 8px 12px; }
.thumb { width: 48px; }
.text { flex-grow: 1; }
button { padding: 6px 12px; border: none; border-radius: 6px; background-color: #3a7bd5; color: #ffffff; }
</style>

<div class="bar">Layers</div>
<div class="stack">{for layer in layers.get() key=layer.0}<section><h2>{layer.1.clone()}</h2><div class="body"><img class="thumb" src="sky.png"><div class="text">{layer.1.clone()} {layer.2} at tick {tick}</div><button>Edit</button></div></section>{/for}</div>
