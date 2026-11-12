<script>
#[prop] let small: Memo<bool> = Memo::new(|| false);
#[prop] let short: Signal<bool> = Signal::new(false);
#[prop] let action: Signal<String> = Signal::new(String::new());
#[prop] let follow: Signal<bool> = Signal::new(false);
let camera_label: Memo<String> = Memo::new(move || if follow.get() { "Camera: Follow".into() } else { "Camera: Fixed".into() });
let camera_toggle = move || action.set(String::from("camera"));
</script>

<style>
.camera-mode { width: 164px; display: block; position: absolute; left: 16px; bottom: 92px; }
.small .camera-mode { left: auto; right: 16px; bottom: 144px; }
.short .camera-mode { left: 8px; right: auto; bottom: 84px; }
</style>

<div class:small={small.get()} class:short={short.get()}>
<button class="camera-mode" data-action="camera" onclick={camera_toggle}>{camera_label}</button>
</div>
