<style>
/* viewport 640x400 */
/* The title attribute on images in the accessibility tree
   (docs/tooltips.md "Not yet built"), as Chromium 155 has it. vapors'
   icon subset refuses title on an svg and takes no img role, so only
   Chromium is recorded: title_images.events.json is compared with
   nothing yet. */
body { margin: 0; font-size: 14px; }
svg { display: block; }
.row { display: flex; align-items: flex-start; gap: 8px; height: 40px; }
</style>

<div class="row"><svg id="svgt" role="img" title="Export (e)" width="12" height="12" viewBox="0 0 12 12"><path d="M2 2H10V10H2Z" fill="none" stroke="currentColor" stroke-width="1.5"/></svg><svg id="svgl" role="img" aria-label="Export" title="Export (e)" width="12" height="12" viewBox="0 0 12 12"><path d="M2 2H10V10H2Z" fill="none" stroke="currentColor" stroke-width="1.5"/></svg><img id="imgempty" alt="" title="Empty alt" src="wide.png" width="20" height="10"><img id="imgnone" title="No alt" src="wide.png" width="20" height="10"><img id="imgalt" alt="Alt" title="Alt title" src="wide.png" width="20" height="10"></div>
