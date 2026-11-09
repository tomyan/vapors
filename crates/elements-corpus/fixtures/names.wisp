<style>
/* viewport 640x400 */
/* Accessible names, values and what is left out of the tree, as
   Chromium's accessibility tree has them (R1): name from content across
   inline and block children and without hidden ones; an empty
   aria-label and an aria-labelledby that finds nothing fall through to
   content; a role list's first valid token; aria-valuetext and
   aria-valuenow on ARIA ranges; the hidden attribute, aria-hidden and
   visibility: hidden leave a node out. Widths are set, as vapors lays
   these controls out only at a width of their own; buttons are as
   tall as their content. The look is not compared (Q1).
   RELAXED: the button rule sets font-family: inherit, font-size:
   inherit and text-align: left, which Chromium's buttons do not have,
   because vapors' ua.css lacks Chromium's button font and centred text;
   with them, text positions match Chromium's. Retire them when ua.css
   gives buttons Chromium's font and centring and the layout oracle
   still passes on this fixture. */
body { margin: 0; font-size: 14px; }
.row { display: flex; align-items: flex-start; height: 40px; }
button, .ctl { box-sizing: border-box; width: 80px; margin: 0; padding: 0; border: 1px solid #555; font-family: inherit; font-size: inherit; text-align: left; }
.ctl { height: 20px; }
.block { display: block; }
.gone { display: none; }
.unseen { visibility: hidden; }
</style>

<div class="row"><button id="save">Sa<b>ve</b></button><button id="open">Op<span class="block">en</span></button><button id="shown">Shown<span class="gone">Gone</span><span class="unseen">Unseen</span></button><button id="empty" aria-label="">Empty</button><button id="nolabel" aria-labelledby="nowhere">Fallback</button><button id="partial" aria-labelledby="nowhere lvl">Unused</button></div><div class="row"><span id="lvl">Le<b>vel</b></span><div id="bogus" class="ctl" role="bogus button" tabindex="0">Bogus</div><div id="level" class="ctl" role="slider" aria-label="Level" aria-valuenow="5" aria-valuetext="five" tabindex="0"></div><div id="count" class="ctl" role="spinbutton" aria-label="Count" aria-valuenow="3" tabindex="0"></div></div><div class="row"><button id="hid" hidden="">Hid</button><button id="ah" aria-hidden="true">AH</button><button id="vh" class="unseen">VH</button><button id="last">Last</button></div>
