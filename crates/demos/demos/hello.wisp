<style>
html, body { height: 100%; }
body {
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #e9ecef;
}
.panel {
  max-width: 440px;
  margin: 16px;
  padding: 20px 24px;
  background-color: #171b1f;
  color: #e4e8ec;
  font-size: 15px;
}
h1 { margin-top: 0; margin-bottom: 8px; font-size: 20px; }
.note { margin-top: 16px; height: 6px; background-color: #3a7bd5; }
</style>

<div class="panel">
  <h1>Hello from vapors</h1>
  <p>This panel is a .wisp component: its CSS cascades through vapors-css, Taffy lays it out, and vapors draws it on the GPU, natively and in this canvas.</p>
  <div class="note"></div>
</div>
