<script>
use crate::layer_list::{LayerList, LayerListProps};

let amount: Signal<i32> = Signal::new(0);
</script>

{snippet badge(label: String)}<em>{label}</em>{/snippet}
{render badge(String::from("first"))}
<LayerList names={vec![(String::from("Curves"), 0), (String::from("Levels"), 1)]}>{snippet row(name: String, index: u32)}<b>{index}: {name} at {amount}</b>{/snippet}</LayerList>
{if amount.get() > 1}{render badge(String::from("more"))}{/if}
{render badge(String::from("last"))}
