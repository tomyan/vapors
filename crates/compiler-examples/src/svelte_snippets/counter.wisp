<script>
#[prop]
let clicks: Snippet<(i32,)> = Snippet::empty();
let count: Signal<i32> = Signal::new(0);
</script>

{render clicks(count)}
<button onclick={move |_| count.update(|n| *n += 1)}>click me</button>
