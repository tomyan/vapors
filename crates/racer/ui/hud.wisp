<script>
use crate::race_stats::{RaceStats, RaceStatsProps};
use crate::race_menu::{RaceMenu, RaceMenuProps};
use crate::camera_toggle::{CameraToggle, CameraToggleProps};
use crate::car_setup::{CarSetup, CarSetupProps};
use crate::driving_controls::{DrivingControls, DrivingControlsProps};
let follow: Signal<bool> = Signal::new(false);
let fps: Signal<String> = Signal::new(String::from("—"));
let speed: Signal<String> = Signal::new(String::from("0"));
let lap: Signal<String> = Signal::new(String::from("1 / 3"));
let clock: Signal<String> = Signal::new(String::from("00:00.00"));
let best: Signal<String> = Signal::new(String::from("—"));
let message: Signal<String> = Signal::new(String::from("Three laps. Keep to the track."));
let action: Signal<String> = Signal::new(String::new());
let paused: Signal<bool> = Signal::new(true);
let settings: Signal<bool> = Signal::new(false);
let short: Signal<bool> = Signal::new(false);
let compact: Signal<bool> = Signal::new(false);
let steering: Signal<f64> = Signal::new(1.0);
let grip: Signal<f64> = Signal::new(1.0);
let size_class: Memo<String> = Memo::new(move || if short.get() { "small short".into() } else if compact.get() { "small".into() } else { "wide".into() });

let left: Signal<bool> = Signal::new(false);
let right: Signal<bool> = Signal::new(false);
let brake: Signal<bool> = Signal::new(false);
let gas: Signal<bool> = Signal::new(false);
let reset: Signal<u64> = Signal::new(0);
let enabled: Signal<bool> = Signal::new(true);
</script>

<style>
:global(html), :global(body) { width: 100%; height: 100%; margin: 0; pointer-events: none; overflow: hidden; }
:global(body) { color: #f3f1e8; font-size: 16px; font-family: sans-serif; }
:global(button) { pointer-events: auto; background-color: #20383d; color: #f3f1e8; border: 1px solid #526a69; border-radius: 5px; padding: 10px 14px; min-height: 44px; font-size: 16px; }
:global(button):hover { background-color: #395456; }
:global(button):focus-visible { outline: 2px solid #f2b26f; outline-offset: 2px; }
.track { position: absolute; left: 0; top: 0; right: 0; bottom: 0; }
.note { position: absolute; left: 16px; top: 98px; padding: 8px 12px; background-color: #182a2f; border-radius: 4px; font-size: 14px; }
.small .note { top: 94px; right: 16px; font-size: 14px; }
.short .note { top: 52px; left: 8px; right: auto; max-width: 300px; padding: 4px 8px; }
</style>

<gpu-surface name="track" class="track"></gpu-surface>
<div class={size_class.get()}>
    <RaceStats layout={size_class} speed={speed} lap={lap} clock={clock} fps={fps} />
    {if !short.get() || paused.get()}<div class="note">{message}</div>{/if}
    {if !settings.get()}<CameraToggle layout={size_class} follow={follow} action={action} />{/if}
    <RaceMenu layout={size_class} paused={paused} action={action} />
    <DrivingControls layout={size_class} left={left} right={right} brake={brake} gas={gas} reset={reset} enabled={enabled} />
    {if settings.get()}<CarSetup layout={size_class} steering={steering} grip={grip} best={best} />{/if}
</div>
