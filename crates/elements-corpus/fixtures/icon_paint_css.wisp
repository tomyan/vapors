<style>
/* viewport 640x400 */
/* paint */
/* An icon's paint from the cascade (one-scene S4a, PR 1): fill, stroke
   and stroke-width are CSS properties, and a path's attributes enter as
   presentation attributes below every author rule, as Chromium ranks
   them. Send drawn four ways in 44 px buttons as icons_paint has them:
   a rule's stroke-width over the attribute's 1.5; a universal rule
   over the attribute; the svg's stroke-width attribute inherited by a
   path without one; and stroke-width in px. */
body { margin: 0; padding: 16px; background-color: #ffffff; }
.row { display: flex; gap: 8px; margin-bottom: 16px; }
button { display: flex; box-sizing: border-box; width: 44px; height: 44px; margin: 0; padding: 0; border: 0; border-radius: 0; background-color: #ececf0; color: #1d1d1f; }
svg { display: block; margin: auto; }
.rule path { stroke-width: 2.5; }
.universal * { stroke-width: 0.75px; }
</style>

<div class="row"><button class="rule" aria-label="Rule"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 8h11M9 4l4 4-4 4" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button class="universal" aria-label="Universal"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 8h11M9 4l4 4-4 4" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button class="inherited" aria-label="Inherited"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" stroke-width="2"><path d="M2 8h11M9 4l4 4-4 4" fill="none" stroke="currentColor"/></svg></button><button class="px" aria-label="Px"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 8h11M9 4l4 4-4 4" fill="none" stroke="currentColor" stroke-width="1px"/></svg></button></div>
