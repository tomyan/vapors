<style>
/* viewport 640x400 */
/* Number inputs for the events oracle (3-E3c), in a row: `amount` at
   x 0..100 (0..1000 by 1, at 20), `plain` at x 100..200 (no min, max
   or value), `off` at x 200..300 disabled, and a button at x 300..380.
   Widths are set, as vapors lays a number field out only at a width
   of its own (Chromium's default is 185 x 24 px at this font size,
   from its font and spin button, which vapors does not model). The
   look is not compared: Chromium's native field and vapors' differ on
   purpose (Q1). */
body { margin: 0; display: flex; font-size: 14px; }
#amount, #plain, #off { box-sizing: border-box; width: 100px; height: 24px; margin: 0; }
button { box-sizing: border-box; width: 80px; height: 24px; margin: 0; padding: 0; border: 1px solid #555; }
</style>

<input id="amount" type="number" min="0" max="1000" step="1" value="20" /><input id="plain" type="number" /><input id="off" type="number" disabled="" /><button id="after">After</button>
