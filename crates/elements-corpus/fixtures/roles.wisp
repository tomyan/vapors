<style>
/* viewport 640x400 */
/* One control of each role the accessibility driver finds (R1), named
   by `aria-label` or content, in a dialog: a group of a range, a number
   and a text field; a region of two checkboxes (one checked and
   disabled), a toggle button, a button and a disabled button; and a
   section with no name, which is no region. Sizes are set, as vapors
   lays these controls out only at a width of their own. The look is not
   compared (Q1). */
body { margin: 0; font-size: 14px; }
.row { display: flex; height: 24px; }
input, button { box-sizing: border-box; height: 24px; margin: 0; padding: 0; }
#amount { width: 200px; }
#radius { width: 80px; }
#note { width: 120px; }
#mono, #lock { width: 13px; height: 13px; margin: 3px; }
button { width: 80px; border: 1px solid #555; }
</style>

<div role="dialog" aria-label="Levels"><div class="row" role="group" aria-label="Tone"><input id="amount" type="range" aria-label="Amount" min="0" max="100" step="1" value="50" /><input id="radius" type="number" aria-label="Radius" min="0" max="10" step="1" value="2" /><input id="note" type="text" aria-label="Note" value="hi" /></div><section class="row" aria-label="Options"><input id="mono" type="checkbox" aria-label="Mono" /><input id="lock" type="checkbox" aria-label="Lock" checked="" disabled="" /><button id="pin" aria-pressed="false">Pin</button><button id="apply">Apply</button><button id="reset" disabled="">Reset</button></section><section class="row"><button id="after">After</button></section></div>
