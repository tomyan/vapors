<style>
/* viewport 640x400 */
/* An uncaptured mouse drag from one element to another (3D 1c PR 5):
   the press is on A, the release on B, neither captures. The click goes
   to their nearest common ancestor, the wrapper. */
body { margin: 0; }
.wrap { width: 200px; height: 40px; display: flex; }
.a { width: 100px; height: 40px; background-color: #88a; }
.b { width: 100px; height: 40px; background-color: #ccd; }
</style>

<div class="wrap" id="wrap"><div class="a" id="a"></div><div class="b" id="b"></div></div>
