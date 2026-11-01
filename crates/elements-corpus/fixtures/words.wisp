<style>
/* viewport 640x400 */
/* A text field for clicks in a row (E6a): `name` at x 0..240 holding
   "Warm tones here". Its width is set, as vapors lays a text field out
   only at a width of its own; its font and border are set, as
   Chromium's field takes neither from the page (its own is 13.33 px
   system-ui in a 2 px border), so a point lands on the same letter in
   both. The look is not compared otherwise (Q1). */
body { margin: 0; display: flex; font-size: 14px; }
#name { box-sizing: border-box; width: 240px; height: 24px; margin: 0; padding: 1px 2px; border: 1px solid #555; font-family: Geist; font-size: 14px; }
</style>

<input id="name" type="text" value="Warm tones here" />
