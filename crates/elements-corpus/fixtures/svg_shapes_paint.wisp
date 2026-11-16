<style>
/* viewport 640x400 */
/* paint */
/* Paths the icon route does not draw, drawn as shapes (one-scene S4a,
   PR 3): fills, a fill with a stroke, and a stroke in a colour other
   than currentColor, in 44 px buttons as icons_paint has them. The
   triangle and the star are meshes; the circle and the rect are closed
   forms, the rect filled and stroked. The notch's reflex corners have
   edges of 0.3 and 0.2 units, shorter than the half-pixel ramp (#371
   verify). The last svg sits at a fractional offset, which the shapes'
   transform keeps (the icon route snaps), and in the last a flex-grown
   svg is drawn at its laid-out size, its view box fitted and centred
   (xMidYMid meet). The second row is S4b's: <rect> square, rounded
   (filled and stroked), with elliptical corners (rx 4, ry 2), and
   with rx 0 and ry 3 (square, as SVG draws one radius of 0), a
   path written as a rounded rect, a filled and a stroked <circle>,
   and a rect over a path. All of these are closed forms except the
   path under the rect. Held against Chromium at 1x and 2x. */
body { margin: 0; padding: 16px; background-color: #ffffff; }
.row { display: flex; gap: 8px; margin-bottom: 16px; }
button { display: flex; box-sizing: border-box; width: 44px; height: 44px; margin: 0; padding: 0; border: 0; border-radius: 0; background-color: #ececf0; color: #1d1d1f; }
svg { display: block; margin: auto; }
.blue path { fill: #2a5fd0; }
.frac svg { margin: 14.25px 13.5px 13.75px 14.5px; }
.grown svg { flex-grow: 1; margin: 14px 4px; }
</style>

<div class="row"><button aria-label="Triangle"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 13L8 3l5 10z" fill="#cc0033"/></svg></button><button aria-label="Circle"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z" fill="#1d1d1f"/></svg></button><button aria-label="Rect"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 4h10v8h-10z" fill="#33aa77" stroke="#112233" stroke-width="1.5"/></svg></button><button aria-label="Star"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M8 1l2 5h5l-4 3 2 5-5-3-5 3 2-5-4-3h5z" fill="#1d1d1f"/></svg></button><button aria-label="Send"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 8h11M9 4l4 4-4 4" fill="none" stroke="#cc0033" stroke-width="1.5"/></svg></button><button class="blue" aria-label="Blue"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 13L8 3l5 10z" fill="#cc0033"/></svg></button><button aria-label="Notch"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 2h12v12h-5.9v-0.3h-0.2v0.3h-5.9z" fill="#1d1d1f"/></svg></button><button class="frac" aria-label="Fraction"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 13L8 3l5 10z" fill="#cc0033"/></svg></button><button class="grown" aria-label="Grown"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 13L8 3l5 10z" fill="#cc0033"/></svg></button></div>
<div class="row"><button aria-label="Box"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><rect x="3" y="4" width="10" height="8" fill="#33aa77"/></svg></button><button aria-label="Rounded"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><rect x="3" y="4" width="10" height="8" rx="2" fill="#33aa77" stroke="#112233" stroke-width="1.5"/></svg></button><button aria-label="Elliptical"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><rect x="2" y="3" width="12" height="10" rx="4" ry="2" fill="#2a5fd0"/></svg></button><button aria-label="Elliptical stroke"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><rect x="2" y="3" width="12" height="10" rx="4" ry="2" fill="none" stroke="#cc0033" stroke-width="1.5"/></svg></button><button aria-label="One zero"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><rect x="3" y="4" width="10" height="8" rx="0" ry="3" fill="#2a5fd0"/></svg></button><button aria-label="Rounded path"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M5 4H11A2 2 0 0 1 13 6V10A2 2 0 0 1 11 12H5A2 2 0 0 1 3 10V6A2 2 0 0 1 5 4Z" fill="#1d1d1f"/></svg></button><button aria-label="Disc"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="5.5" fill="#1d1d1f"/></svg></button><button aria-label="Ring"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="5" fill="none" stroke="#cc0033" stroke-width="1.5"/></svg></button><button aria-label="Over"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 13L8 3l5 10z" fill="#cc0033"/><rect x="6" y="8" width="6" height="5" rx="1" fill="#2a5fd0"/></svg></button></div>
