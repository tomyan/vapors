<script>
use vapors_components::hold_button::{HoldButton, HoldButtonProps};
#[prop] let small: Memo<bool> = Memo::new(|| false);
#[prop] let short: Signal<bool> = Signal::new(false);
#[prop] let left: Signal<bool> = Signal::new(false);
#[prop] let right: Signal<bool> = Signal::new(false);
#[prop] let brake: Signal<bool> = Signal::new(false);
#[prop] let gas: Signal<bool> = Signal::new(false);
#[prop] let reset: Signal<u64> = Signal::new(0);
#[prop] let enabled: Signal<bool> = Signal::new(true);
</script>

<style>
.drive { position: absolute; bottom: 16px; left: 16px; right: 16px; display: flex; justify-content: space-between; }
.pair { display: flex; gap: 8px; }
.help { position: absolute; bottom: 26px; left: 200px; font-size: 14px; color: #f3f1e8; background-color: #182a2f; padding: 8px 12px; border-radius: 4px; }
.small .help { display: none; }
.short .drive { bottom: 8px; left: 8px; right: 8px; }
.drive { --hold-width: 68px; }
.small .drive { --hold-width: 58px; }
.gas { --hold-background: #b9d7bc; --hold-color: #172b30; }

</style>

<div class:small={small.get()} class:short={short.get()}>
<div class="drive">
    <div class="pair">
        <HoldButton id={String::from("left")} label={String::from("←")} on_change={vapors_components::hold_into(left)} reset={reset} enabled={enabled} />
        <HoldButton id={String::from("right")} label={String::from("→")} on_change={vapors_components::hold_into(right)} reset={reset} enabled={enabled} />
    </div>
    <div class="pair">
        <HoldButton id={String::from("brake")} label={String::from("Brake")} on_change={vapors_components::hold_into(brake)} reset={reset} enabled={enabled} />
        <div class="gas"><HoldButton id={String::from("gas")} label={String::from("Gas")} on_change={vapors_components::hold_into(gas)} reset={reset} enabled={enabled} /></div>
    </div>
</div>
<div class="help">WASD / arrows · Space pauses · R restarts</div>
</div>
