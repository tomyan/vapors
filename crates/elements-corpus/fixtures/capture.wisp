<style>
/* viewport 640x400 */
/* Pointer capture for the events oracle (3-E1): the thumb captures the
   pointer on pointerdown (the script's `capture`), the track does not.
   A mouse drag and a touch drag each leave the thumb; a touch on the
   track is captured there without asking. */
body { margin: 0; }
.thumb { width: 100px; height: 40px; background-color: #88a; }
.track { width: 100px; height: 40px; background-color: #ccd; }
</style>

<div class="thumb" id="thumb"></div>
<div class="track" id="track"></div>
