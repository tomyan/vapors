<style>
/* viewport 640x400 */
/* paint */
/* Outlines (3-E2b), drawn by vapors and by Chromium: a ring outside
   the border box at its offset, not laid out; following the border's
   rounded corners; under a positioned box with a z-index that
   overlaps it (.over on .under's top-left corner), as an outline is
   painted with its own layer, not above every layer. The ring lies
   outside each element's region, so paint_check's flat-region and
   all-pixel rules hold it, not the per-region rule. */
body { margin: 0; padding: 20px; display: flex; gap: 40px; align-items: flex-start; background-color: #ffffff; }
div { width: 80px; height: 60px; background-color: #dde3ea; }
.plain { outline: 2px solid #c0392b; }
.offset { outline: 3px solid #1f6feb; outline-offset: 6px; }
.round { border-radius: 12px; outline: 2px solid #2d8a4e; outline-offset: 4px; }
.tight { border: 2px solid #555555; outline: 4px solid #8e44ad; }
.under { outline: 4px solid #c0392b; }
.over { position: absolute; left: 490px; top: 10px; width: 50px; height: 50px; z-index: 1; background-color: #f5b041; }
</style>

<div class="plain"></div><div class="offset"></div><div class="round"></div><div class="tight"></div><div class="under"></div><div class="over"></div>
