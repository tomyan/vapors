<script>
// vapors' virtual list given no height, inside a panel below a 30 px
// header that reaches 10 px above the window's bottom: the list fills the
// panel, and its rows follow the panel's laid-out height as the window
// resizes. 100 rows of 20 px, one row of overscan.
use vapors_components::virtual_list::{VirtualList, VirtualListProps};
#[prop] let keys: Signal<Vec<usize>> = Signal::new((0..100).collect());
</script>

<style>
.head { height: 30px; }
.panel { position: absolute; top: 30px; bottom: 10px; left: 0; width: 100px; }
</style>

<div class="head"></div>
<div class="panel">
  <VirtualList keys={keys} row_height={20.0} overscan={1}>{snippet row(key: usize)}<div class="row">{key}</div>{/snippet}</VirtualList>
</div>
