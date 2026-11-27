<style>
/* viewport 800x100 */
/* The gui-bench top bar's live switch (`ui/topbar.wisp`): its rules, placed
   by `left` here where the top bar uses `right`, at its spec position,
   with the aria-label it has there; the knob is moved by `:checked +`, as
   there; a second, checked one is the state after a click. Each is a
   checkbox with `appearance: none` whose background is the track, and
   its knob a decorative `aria-hidden` sibling that takes no pointer
   (docs/appearance.md). The oracle holds the boxes, a click on the knob
   and Space, the role, name and checked state to Chromium, and that the
   knobs are not in the tree; the painted look is held by gui-bench's
   tests/switch_knob.rs. */
body { margin: 0; height: 64px; position: relative; }
.live-switch { display: block; position: absolute; top: 21px; width: 40px; height: 22px; margin: 0; appearance: none; border-radius: 11px; background-color: #d1d5db; }
.live-switch:checked { background-color: #4f46e5; }
.live-knob { position: absolute; top: 23px; width: 18px; height: 18px; border-radius: 9px; background-color: #ffffff; box-shadow: 0 1px 3px #11182740; pointer-events: none; }
#live { left: 736px; }
#live + .live-knob { left: 738px; }
#live:checked + .live-knob { left: 756px; }
#on { left: 600px; }
#on + .live-knob { left: 602px; }
#on:checked + .live-knob { left: 620px; }
</style>

<input id="live" class="live-switch" type="checkbox" switch="" aria-label="Live updates" /><div class="live-knob" aria-hidden="true"></div><input id="on" class="live-switch" type="checkbox" switch="" checked="" aria-label="Live updates on" /><div class="live-knob" aria-hidden="true"></div>
