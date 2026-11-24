<script>
// 20 rows of 20 px in vapors' virtual list, 60 px tall with one row of
// overscan: only the rows in and next to the window exist, each keyed by
// the number it shows, which the test reorders through `keys`.
use vapors_components::virtual_list::{VirtualList, VirtualListProps};
#[prop] let keys: Signal<Vec<usize>> = Signal::new((0..20).collect());
</script>

<style>
.frame { width: 100px; }
</style>

<div class="frame">
  <VirtualList keys={keys} row_height={20.0} height={60.0} overscan={1}>{snippet row(key: usize)}<div class="row">{key}</div>{/snippet}</VirtualList>
</div>
