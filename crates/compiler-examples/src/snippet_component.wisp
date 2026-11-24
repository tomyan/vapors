<script>
// A component tag inside a snippet body: one passed as a prop, one
// rendered in place.
use crate::layer_list::{LayerList, LayerListProps};
use crate::required_label::{RequiredLabel, RequiredLabelProps};
</script>

{snippet badge(size: usize)}<RequiredLabel label={String::from("badge")} size={size.get()} />{/snippet}
<LayerList names={vec![(String::from("Curves"), 2)]}>{snippet row(name: String, index: u32)}<RequiredLabel label={name.get()} size={index.get() as usize} />{/snippet}</LayerList>
{render badge(3)}
