<script>
let fps: Signal<String> = Signal::new(String::from("—"));
let speed: Signal<String> = Signal::new(String::from("0"));
let lap: Signal<String> = Signal::new(String::from("1 / 3"));
let clock: Signal<String> = Signal::new(String::from("00:00.00"));
let best: Signal<String> = Signal::new(String::from("—"));
let message: Signal<String> = Signal::new(String::from("Three laps. Keep to the track."));
let action: Signal<String> = Signal::new(String::new());
let paused: Signal<bool> = Signal::new(true);
let settings: Signal<bool> = Signal::new(false);
let compact: Signal<bool> = Signal::new(false);
let steering: Signal<f64> = Signal::new(1.0);
let grip: Signal<f64> = Signal::new(1.0);
let size_class: Memo<String> = Memo::new(move || if compact.get() { "small".into() } else { "wide".into() });
let pause_label: Memo<String> = Memo::new(move || if paused.get() { "Drive".into() } else { "Pause".into() });
let toggle = move || action.set(String::from("toggle"));
let restart = move || action.set(String::from("restart"));
let tune = move || action.set(String::from("tune"));
</script>

<style>
:global(html), :global(body) { width: 100%; height: 100%; margin: 0; pointer-events: none; overflow: hidden; }
:global(body) { color: #f3f1e8; font-size: 16px; font-family: sans-serif; }
.top { position: absolute; top: 16px; left: 16px; right: 16px; display: flex; justify-content: space-between; align-items: flex-start; gap: 8px; }
.brand { padding: 10px 14px; background-color: #182a2f; border-radius: 6px; }
.name { font-size: 20px; font-weight: 700; }
.caption { font-size: 14px; color: #a9c3be; }
.stats { display: flex; gap: 18px; padding: 10px 16px; border-radius: 6px; background-color: #182a2f; }
.stat { min-width: 55px; }
.value { font-size: 22px; font-weight: 700; }
.note { position: absolute; left: 16px; top: 98px; padding: 8px 12px; background-color: #182a2f; border-radius: 4px; font-size: 14px; }
.menu { position: absolute; right: 16px; bottom: 92px; display: flex; gap: 8px; }
button { pointer-events: auto; background-color: #20383d; color: #f3f1e8; border: 1px solid #526a69; border-radius: 5px; padding: 10px 14px; min-height: 44px; font-size: 16px; }
button:hover { background-color: #395456; }
button:focus-visible { outline: 2px solid #f2b26f; outline-offset: 2px; }
.primary { background-color: #e5ac69; color: #172b30; border-color: #e5ac69; font-weight: 700; }
.drive { position: absolute; bottom: 16px; left: 16px; right: 16px; display: flex; justify-content: space-between; }
.pair { display: flex; gap: 8px; }
.pedal { width: 68px; height: 60px; padding: 6px; font-weight: 700; }
.gas { background-color: #b9d7bc; color: #172b30; }
.help { position: absolute; bottom: 26px; left: 200px; font-size: 14px; color: #f3f1e8; background-color: #182a2f; padding: 8px 12px; border-radius: 4px; }
.tuning { pointer-events: auto; position: absolute; right: 16px; top: 148px; width: 270px; padding: 16px; border-radius: 8px; background-color: #182a2f; border: 1px solid #526a69; }
h2 { margin: 0 0 12px; font-size: 18px; }
.row { display: flex; align-items: center; gap: 10px; margin: 8px 0 14px; }
input { pointer-events: auto; font-size: 16px; height: 32px; }
input[type=range] { width: 170px; --vapors-thumb-color: #e5ac69; --vapors-track-color: #526a69; }
input[type=number] { width: 70px; color: #172b30; background-color: #f3f1e8; }
input:invalid { border: 2px solid #ee806b; }
.small .brand { display: none; }
.small .stats { flex-grow: 1; justify-content: space-between; gap: 4px; padding: 8px; }
.small .stat { min-width: 40px; }
.small .value { font-size: 19px; }
.small .note { top: 94px; right: 16px; font-size: 14px; }
.small .help { display: none; }
.small .pedal { width: 58px; }
</style>

<div class={size_class.get()}>
<div class="top"><div class="brand"><div class="name">VAPORS / CIRCUIT</div><div class="caption">Rust · compiled components · GPU</div></div><div class="stats"><div class="stat"><div class="caption">KM/H</div><div class="value">{speed}</div></div><div class="stat"><div class="caption">LAP</div><div class="value">{lap}</div></div><div class="stat"><div class="caption">TIME</div><div class="value">{clock}</div></div><div class="stat"><div class="caption">FPS</div><div class="value">{fps}</div></div></div></div>
<div class="note">{message}</div>
<div class="menu"><button data-action="tune" onclick={tune}>Tune</button><button data-action="restart" onclick={restart}>Restart</button><button class="primary" data-action="toggle" onclick={toggle}>{pause_label}</button></div>
<div class="drive"><div class="pair"><button class="pedal" data-drive="left">←</button><button class="pedal" data-drive="right">→</button></div><div class="pair"><button class="pedal" data-drive="brake">Brake</button><button class="pedal gas" data-drive="gas">Gas</button></div></div>
<div class="help">WASD / arrows · Space pauses · R restarts</div>
{if settings.get()}<div class="tuning"><h2>Car setup</h2><div>Steering</div><div class="row"><input type="range" min="0.5" max="1.5" step="0.05" bind:value={steering}><input type="number" data-setting="steering" min="0.5" max="1.5" step="0.05" bind:value={steering}></div><div>Grip</div><div class="row"><input type="range" min="0.5" max="1.5" step="0.05" bind:value={grip}><input type="number" data-setting="grip" min="0.5" max="1.5" step="0.05" bind:value={grip}></div><div class="caption">Best lap: {best}</div><div class="caption">Changes apply to this run.</div></div>{/if}
</div>
