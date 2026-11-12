<script>
#[prop]
let on: Signal<bool> = Signal::new(false);
</script>

<div class="card" class:lit={on.get()}></div>
