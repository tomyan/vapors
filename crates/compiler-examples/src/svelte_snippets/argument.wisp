<script>
let count: Signal<i32> = Signal::new(0);
</script>

{snippet foo(n: i32)}<p>clicks: {n}</p>{/snippet}
{render foo(count)}
<button onclick={move |_| count.update(|n| *n += 1)}>click me</button>
