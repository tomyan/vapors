<script>
// The racer's `track` surface alone, for the racer's GPU tests
// (`src/gpu_world_rig.rs`): 320 x 240, 240 x 320 with `portrait`, or the
// whole window with `window`.
#[prop] let portrait: bool = false;
#[prop] let window: bool = false;
</script>

<style>
:global(html), :global(body) { margin: 0; width: 100%; height: 100%; }
.view { display: block; width: 320px; height: 240px; }
.view.portrait { width: 240px; height: 320px; }
.view.window { width: 100%; height: 100%; }
</style>

<gpu-surface name="track" class="view" class:portrait={portrait} class:window={window}></gpu-surface>
