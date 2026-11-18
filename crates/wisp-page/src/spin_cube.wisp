<script>
use crate::taps::Faces;

#[prop]
let faces: std::rc::Rc<Faces> = std::rc::Rc::default();
let turns: Signal<u32> = Signal::new(0);
</script>

<style>
.cube { color: red; }
</style>

<div class="cube">{turns} {faces.0.len()}</div>
