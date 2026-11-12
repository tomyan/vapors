<script>
#[prop]
let layers: Signal<Vec<(i32, i32)>> = Signal::new(Vec::new());
</script>

{snippet readout(id: i32, amount: i32)}<b>{id}={amount}</b>{/snippet}
<ul>{for layer in layers.get() key=layer.0}<li>{render readout(layer.0, layer.1)}</li>{/for}</ul>
