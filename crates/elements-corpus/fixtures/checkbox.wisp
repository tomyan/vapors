<style>
/* viewport 640x400 */
/* Checkboxes for the events oracle (3-E3b), in a row: `visible`
   unsized (Chromium's default 13 x 13 box with its margins), `hidden`
   checked, `off` disabled, each after a 40 px spacer, and a button. The
   row is 40 px tall, so an unsized checkbox shows whether it stretches
   as a flex item. The look is not compared: Chromium's native box and
   vapors' part differ on purpose (Q1). */
body { margin: 0; display: flex; height: 40px; }
.gap { width: 40px; }
button { box-sizing: border-box; width: 80px; height: 20px; margin: 0; padding: 0; border: 1px solid #555; }
</style>

<input id="visible" type="checkbox" /><div class="gap"></div><input id="hidden" type="checkbox" checked="" /><div class="gap"></div><input id="off" type="checkbox" disabled="" /><div class="gap"></div><button id="after">After</button>
