<script>
#[prop]
let presses: Signal<u32> = Signal::new(0);
let count = move || presses.update(|n| *n += 1);
</script>

<vapors:window onkeyup={count} />
