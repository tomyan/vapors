<script>
use super::counter::{Counter, CounterProps};
</script>

{snippet clicks(n: i32)}<p>clicks: {n}</p>{/snippet}
<Counter clicks={clicks.clone()} />
