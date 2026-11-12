<script>
use crate::card::{Card, CardProps};

#[prop]
let layers: Signal<Vec<(i32, String)>> = Signal::new(Vec::new());
</script>

{snippet amount(value: i32)}<i>{value}</i>{/snippet}
{for layer in layers.get() key=layer.0}<Card title={layer.get().1}>{render amount(layer.0)}</Card>{/for}
