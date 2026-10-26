<style>
/* viewport 640x400 */
/* paint */
/* Outlines (3-E2b), drawn by vapors and by Chromium: a ring outside
   the border box at its offset, not laid out; following the border's
   rounded corners; drawn over a later sibling it overlaps. */
body { margin: 0; padding: 20px; display: flex; gap: 40px; align-items: flex-start; background-color: #ffffff; }
div { width: 80px; height: 60px; background-color: #dde3ea; }
.plain { outline: 2px solid #c0392b; }
.offset { outline: 3px solid #1f6feb; outline-offset: 6px; }
.round { border-radius: 12px; outline: 2px solid #2d8a4e; outline-offset: 4px; }
.tight { border: 2px solid #555555; outline: 4px solid #8e44ad; }
</style>

<div class="plain"></div><div class="offset"></div><div class="round"></div><div class="tight"></div>
