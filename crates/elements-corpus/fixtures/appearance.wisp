<style>
/* viewport 400x100 */
/* `appearance: none` on checkboxes (docs/appearance.md), in a row from
   the top: `bare` unsized, which Chromium lays out with no content (0 x 0
   inside its margins); `track` a 40 x 22 switch whose own background and
   radius are its track; `on` the same, checked. The border is set to 0
   because vapors' 1px border is its user-agent look of a checkbox, where
   Chromium draws its look in the theme, which `none` takes away. Chromium
   157 has no `switch` attribute, so its `track` and `on` are checkboxes
   (docs/switch.md); the oracle holds the boxes, roles, names and states. */
body { margin: 0; display: flex; align-items: flex-start; }
input { appearance: none; border: 0; }
.track { width: 40px; height: 22px; border-radius: 11px; background-color: #d1d5db; }
.track:checked { background-color: #4f46e5; }
</style>

<input id="bare" type="checkbox" aria-label="Bare" /><input id="track" class="track" type="checkbox" switch="" aria-label="Track" /><input id="on" class="track" type="checkbox" switch="" checked="" aria-label="On" />
