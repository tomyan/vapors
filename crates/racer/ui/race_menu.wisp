<script>
#[prop] let short: Signal<bool> = Signal::new(false);
#[prop] let action: Signal<String> = Signal::new(String::new());
#[prop] let paused: Signal<bool> = Signal::new(true);
let pause_label: Memo<String> = Memo::new(move || if paused.get() { "Drive".into() } else { "Pause".into() });
let toggle = move || action.set(String::from("toggle"));
let restart = move || action.set(String::from("restart"));
let tune = move || action.set(String::from("tune"));
</script>

<style>
.menu { position: absolute; right: 16px; bottom: 92px; display: flex; gap: 8px; }
.primary { background-color: #e5ac69; color: #172b30; border-color: #e5ac69; font-weight: 700; }
.short .menu { right: 8px; bottom: 84px; gap: 6px; }
</style>

<div class:short={short.get()}>
    <div class="menu">
        <button data-action="tune" onclick={tune}>Tune</button>
        <button data-action="restart" onclick={restart}>Restart</button>
        <button class="primary" data-action="toggle" onclick={toggle}>{pause_label}</button>
    </div>
</div>
