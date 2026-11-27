<style>
/* viewport 800x100 */
/* The gui-bench top bar's live switch (`ui/topbar.wisp`): the same rule for
   the `switch` input, at its spec position, with the aria-label it has
   there; a second, checked one is the state after a click. The oracle
   holds the box, the click and Space, the role, name and checked state to
   Chromium; the knob's look and shadow are not compared (Chromium's
   native switch and vapors' part differ on purpose, Q1). */
body { margin: 0; height: 64px; position: relative; }
.live-switch { display: block; position: absolute; top: 21px; width: 40px; height: 22px; margin: 0; accent-color: #4f46e5; }
#live { left: 736px; }
#on { left: 600px; }
</style>

<input id="live" class="live-switch" type="checkbox" switch="" aria-label="Live updates" /><input id="on" class="live-switch" type="checkbox" switch="" checked="" aria-label="Live updates on" />
