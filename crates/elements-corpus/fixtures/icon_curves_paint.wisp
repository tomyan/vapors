<style>
/* viewport 640x400 */
/* paint */
/* Curves in icons (docs/icons.md, the subset; one-scene S2: C and Q in
   the parser), drawn by vapors and by Chromium: four 16 px icons whose
   paths use cubic (C, c) and quadratic (Q, q) curves, in 44 px buttons
   as icons_paint has them, dark strokes on a light button and light on
   a dark one. zeno flattens the curves before stroking, as it does arcs;
   this fixture holds that against Chromium at 1x and 2x. */
body { margin: 0; padding: 16px; background-color: #ffffff; }
.row { display: flex; gap: 8px; margin-bottom: 16px; }
button { display: flex; box-sizing: border-box; width: 44px; height: 44px; margin: 0; padding: 0; border: 0; border-radius: 0; }
svg { display: block; margin: auto; }
.dark { background-color: #ececf0; color: #1d1d1f; }
.light { background-color: #1d1d1f; color: #f5f5f7; }
</style>

<div class="row dark-row"><button class="dark wave" aria-label="Wave"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 8C4 3 6 3 8 8c2 5 4 5 6 0" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button class="dark bubble" aria-label="Bubble"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 3h10q1 0 1 1v6q0 1-1 1H7l-3 3v-3H3q-1 0-1-1V4q0-1 1-1z" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button class="dark heart" aria-label="Heart"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M8 14C4 10 2 8 2 5.5 2 3.5 3.5 2 5.5 2 6.5 2 7.5 2.5 8 3.5 8.5 2.5 9.5 2 10.5 2 12.5 2 14 3.5 14 5.5 14 8 12 10 8 14z" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button class="dark tick" aria-label="Tick"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 9Q5 10 7 13Q9 6 14 3" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button></div>
<div class="row light-row"><button class="light wave" aria-label="Wave"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 8C4 3 6 3 8 8c2 5 4 5 6 0" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button class="light bubble" aria-label="Bubble"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 3h10q1 0 1 1v6q0 1-1 1H7l-3 3v-3H3q-1 0-1-1V4q0-1 1-1z" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button class="light heart" aria-label="Heart"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M8 14C4 10 2 8 2 5.5 2 3.5 3.5 2 5.5 2 6.5 2 7.5 2.5 8 3.5 8.5 2.5 9.5 2 10.5 2 12.5 2 14 3.5 14 5.5 14 8 12 10 8 14z" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button class="light tick" aria-label="Tick"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 9Q5 10 7 13Q9 6 14 3" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button></div>
