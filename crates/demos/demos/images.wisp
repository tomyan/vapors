<style>
/* Images (F2c S10): a history list whose entries show a thumbnail of
   the image after that turn, decoded from bundled PNGs and registered
   with the document by name. Each 96 x 64 image is drawn at 48 x 32. */
:global(html), :global(body) { height: 100%; }
:global(body) { margin: 0; background-color: #e9ecef; font-size: 14px; color: #171b1f; }
.bar { height: 44px; display: flex; align-items: center; padding: 0 12px; background-color: #171b1f; color: #e4e8ec; }
.entry { display: flex; align-items: center; gap: 12px; padding: 8px 12px; border-bottom: 1px solid #c7ccd1; background-color: #ffffff; }
.thumb { width: 48px; }
</style>

<div class="bar">History</div>
<div class="entry"><img class="thumb" src="sky.png"><div>Turn 1: opened, sky and grass</div></div>
<div class="entry"><img class="thumb" src="warm.png"><div>Turn 2: warmed the whole image</div></div>
<div class="entry"><img class="thumb" src="mono.png"><div>Turn 3: black and white</div></div>
