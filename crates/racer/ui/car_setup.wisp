<script>
#[prop] let layout: Memo<String> = Memo::new(|| String::from("wide"));
#[prop] let steering: Signal<f64> = Signal::new(1.0);
#[prop] let grip: Signal<f64> = Signal::new(1.0);
#[prop] let best: Signal<String> = Signal::new(String::new());
</script>

<style>
.caption { font-size: 14px; color: #a9c3be; }
.tuning { pointer-events: auto; position: absolute; right: 16px; top: 148px; width: 270px; padding: 16px; border-radius: 8px; background-color: #182a2f; border: 1px solid #526a69; }
.row { display: flex; align-items: center; gap: 10px; margin: 8px 0 14px; }
.short .tuning { top: 48px; right: 8px; padding: 8px; }
.short .tuning h2 { display: none; }
.short .tuning .row { margin: 2px 0 4px; }
h2 { margin: 0 0 12px; font-size: 18px; }
input { pointer-events: auto; font-size: 16px; height: 32px; }
input[type=range] { width: 170px; --vapors-thumb-color: #e5ac69; --vapors-track-color: #526a69; }
input[type=number] { width: 70px; color: #172b30; background-color: #f3f1e8; }
input:invalid { border: 2px solid #ee806b; }
</style>

<div class={layout.get()}>
    <div class="tuning">
        <h2>Car setup</h2>
        <div>Steering</div>
        <div class="row">
            <input type="range" min="0.5" max="1.5" step="0.05" bind:value={steering}>
            <input type="number" data-setting="steering" min="0.5" max="1.5" step="0.05" bind:value={steering}>
        </div>
        <div>Grip</div>
        <div class="row">
            <input type="range" min="0.5" max="1.5" step="0.05" bind:value={grip}>
            <input type="number" data-setting="grip" min="0.5" max="1.5" step="0.05" bind:value={grip}>
        </div>
        <div class="caption">Best lap: {best}</div>
        <div class="caption">Changes apply to this run.</div>
    </div>
</div>
