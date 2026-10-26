<style>
/* viewport 640x400 */
/* Buttons for the events oracle (11-Q1, D3): Save and Cancel take clicks
   and focus, Delete is disabled and takes neither; a list scrolls under
   the wheel. Sizes are set, so the browser's button styles do not decide
   the boxes. The bar is a flex row: vapors lays out a button only as a
   flex or grid item. No input or a: vapors lays out neither yet (F4, F2d). */
body { margin: 8px; font-size: 14px; }
.bar { display: flex; }
button { box-sizing: border-box; width: 90px; height: 32px; margin-right: 8px; padding: 0; border: 1px solid #555; background-color: #eee; font-size: 14px; }
.list { width: 200px; height: 80px; margin-top: 12px; overflow: auto; background-color: #eef; }
.row { height: 30px; margin-bottom: 4px; background-color: #ccd; }
</style>

<div class="bar"><button id="save">Save</button><button id="cancel">Cancel</button><button id="delete" disabled="">Delete</button></div>
<div class="list" id="list"><div class="row"></div><div class="row"></div><div class="row"></div><div class="row"></div></div>
