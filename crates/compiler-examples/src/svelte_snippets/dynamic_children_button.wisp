<script>
#[prop]
let children: Snippet = Snippet::empty();
#[prop]
let show: Signal<bool> = Signal::new(false);
</script>

<button onclick={move |_| show.set(true)}>{render children()}</button>
