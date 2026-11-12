<style>
/* viewport 640x400 */
/* Text fields for the events oracle (E6a), in a row: `name` at x
   0..160 (its text "Warm", 14 px), `plain` at x 160..320 empty, `off`
   at x 320..480 disabled, a button at x 480..560. Widths are set, as
   vapors lays a text field out only at a width of its own. The look is
   not compared: Chromium's native field and vapors' differ on purpose
   (Q1). */
body { margin: 0; display: flex; font-size: 14px; }
#name, #plain, #off { box-sizing: border-box; width: 160px; height: 24px; margin: 0; padding: 1px 2px; }
button { box-sizing: border-box; width: 80px; height: 24px; margin: 0; padding: 0; border: 1px solid #555; }
</style>

<input id="name" type="text" value="Warm" /><input id="plain" type="text" /><input id="off" type="text" disabled="" value="off" /><button id="after">After</button>
