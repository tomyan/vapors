<style>
/* viewport 640x400 */
/* A range input for the events oracle (3-E3a), in a row: `level` at
   x 0..200 (0..100 by 1, at 20), `plain` unsized (Chromium's default
   size), `off` disabled at x 333..533, and a button after them. Sizes
   are set on the first and the last so a press lands at a known value
   in both browsers: the thumb's 16 px is the same, so a point maps to
   the same value. The look is not compared: Chromium's native slider
   and vapors' --vapors-* parts differ on purpose (Q1). */
body { margin: 0; display: flex; }
#level, #off { box-sizing: border-box; width: 200px; height: 20px; margin: 0; padding: 0; }
button { box-sizing: border-box; width: 80px; height: 20px; margin: 0; padding: 0; border: 1px solid #555; }
</style>

<input id="level" type="range" min="0" max="100" step="1" value="20" /><input id="plain" type="range" /><input id="off" type="range" disabled="" /><button id="after">After</button>
