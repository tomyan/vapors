<style>
/* viewport 640x400 */
/* Focus and keys for the events oracle (3-E2a): two buttons and two
   tabindex boxes in the Tab order, a disabled button and a tabindex=-1
   box out of it. Sizes are set, so the browser's button styles do not
   decide the boxes; buttons are flex items, as vapors lays a button out
   only as a flex or grid item. */
body { margin: 0; display: flex; flex-wrap: wrap; }
button, div { box-sizing: border-box; width: 100px; height: 40px; margin: 0; padding: 0; border: 1px solid #555; background-color: #eee; font-size: 14px; }
</style>

<button id="save">Save</button><div id="note" tabindex="0"></div><button id="gone" disabled="">Gone</button><div id="aside" tabindex="-1"></div><button id="cancel">Cancel</button>
