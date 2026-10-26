<script>
use super::counter::{Counter, CounterProps};
</script>

{snippet foo(n: i32)}<p>clicks: {n}</p>{/snippet}
<Counter foo={foo.clone()} />
