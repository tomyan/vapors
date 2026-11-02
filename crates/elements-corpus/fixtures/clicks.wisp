<style>
/* viewport 640x400 */
/* One button for the events oracle's click counts and modifiers (for
   touché's tool card: a label's double click or Alt-click resets its
   control). Sized, so the browser's button styles do not decide its box;
   a flex row, as vapors lays out a button only as a flex or grid item. */
body { margin: 8px; font-size: 14px; }
.bar { display: flex; }
button { box-sizing: border-box; width: 90px; height: 32px; padding: 0; border: 1px solid #555; background-color: #eee; font-size: 14px; }
</style>

<div class="bar"><button id="reset">Reset</button></div>
