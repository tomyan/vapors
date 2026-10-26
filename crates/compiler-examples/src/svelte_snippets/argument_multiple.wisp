<script>
let count: Signal<i32> = Signal::new(0);
let doubled: Memo<i32> = Memo::new(move || count.get() * 2);
</script>

{snippet foo(n: i32, twice: i32)}<p>clicks: {n}, doubled: {twice}</p>{/snippet}
{render foo(count, doubled)}
<button onclick={move |_| count.update(|n| *n += 1)}>click me</button>
