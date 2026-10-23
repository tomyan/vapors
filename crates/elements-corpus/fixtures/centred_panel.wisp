<style>
/* viewport 640x400 */
html, body { height: 100%; }
body { margin: 0; display: flex; align-items: center; justify-content: center; }
.panel { max-width: 440px; margin: 16px; padding: 20px 24px; font-size: 15px; background-color: #171b1f; }
h1 { margin-top: 0; margin-bottom: 8px; font-size: 20px; }
.half { height: 50%; width: 10px; }
</style>

<div class="panel">
  <h1>Hello from vapors</h1>
  <p>This panel is a .wisp component: its CSS cascades through vapors-css, Taffy lays it out, and vapors draws it on the GPU, natively and in this canvas.</p>
</div>
<div class="half"></div>
