<script>
use crate::layer_list::{LayerList, LayerListProps};
</script>

{snippet b(name: String, index: u32)}<i>{index}{name}</i>{/snippet}
<LayerList names={vec![(String::from("a"), 0)]} row={b} />
{render b(String::from("z"), 9)}
