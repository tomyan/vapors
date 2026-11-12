<script>
#[prop] let layout: Memo<String> = Memo::new(|| String::from("wide"));
#[prop] let speed: Signal<String> = Signal::new(String::new());
#[prop] let lap: Signal<String> = Signal::new(String::new());
#[prop] let clock: Signal<String> = Signal::new(String::new());
#[prop] let fps: Signal<String> = Signal::new(String::new());
</script>

<style>
.top { position: absolute; top: 16px; left: 16px; right: 16px; display: flex; justify-content: space-between; align-items: flex-start; gap: 8px; }
.brand { padding: 10px 14px; background-color: #182a2f; border-radius: 6px; }
.name { font-size: 20px; font-weight: 700; }
.caption { font-size: 14px; color: #a9c3be; }
.stats { display: flex; gap: 18px; padding: 10px 16px; border-radius: 6px; background-color: #182a2f; }
.stat { min-width: 55px; }
.value { font-size: 22px; font-weight: 700; }
.small .brand { display: none; }
.small .stats { flex-grow: 1; justify-content: space-between; gap: 4px; padding: 8px; }
.small .stat { min-width: 40px; }
.small .value { font-size: 19px; }
.short .top { top: 8px; left: 8px; right: 8px; }
.short .stats { padding: 6px 10px; }
.short .stat { display: flex; align-items: center; gap: 8px; }
.short .value { font-size: 18px; }
</style>

<div class={layout.get()}>
    <div class="top">
        <div class="brand">
            <div class="name">VAPORS / CIRCUIT</div>
            <div class="caption">Rust · compiled components · GPU</div>
        </div>
        <div class="stats">
            <div class="stat">
                <div class="caption">KM/H</div>
                <div class="value">{speed}</div>
            </div>
            <div class="stat">
                <div class="caption">LAP</div>
                <div class="value">{lap}</div>
            </div>
            <div class="stat">
                <div class="caption">TIME</div>
                <div class="value">{clock}</div>
            </div>
            <div class="stat">
                <div class="caption">FPS</div>
                <div class="value">{fps}</div>
            </div>
        </div>
    </div>
</div>
