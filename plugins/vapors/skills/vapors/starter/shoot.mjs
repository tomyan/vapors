// node shoot.mjs <url> <out-prefix>
// Loads a vapors page in headless Chromium with software WebGPU, saves
// <out-prefix>-1.png after 4 s, presses ArrowLeft and ArrowUp, saves
// <out-prefix>-2.png. Exits 1 when the page shows an error in #error.
// Needs the `playwright` (or `playwright-core`) npm package; set CHROME
// to use a browser other than Playwright's own Chromium.
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';

const { chromium } = await import('playwright').catch(() => import('playwright-core'));
const [url, out] = process.argv.slice(2);
if (!url || !out) throw new Error('usage: node shoot.mjs <url> <out-prefix>');

// Without a GPU, WebGPU runs on the browser's own SwiftShader Vulkan
// driver; with other flags the device is lost within two frames
// (https://github.com/tomyan/vapors/issues/513).
const executable = process.env.CHROME || chromium.executablePath();
const icd = process.env.VK_DRIVER_FILES || join(dirname(executable), 'vk_swiftshader_icd.json');
if (!existsSync(icd)) throw new Error(`no ${icd}; set VK_DRIVER_FILES to the browser's vk_swiftshader_icd.json`);
const env = { ...process.env, VK_DRIVER_FILES: icd };
// a session bus with no portal stalls every load for 120 s
delete env.DBUS_SESSION_BUS_ADDRESS;

const browser = await chromium.launch({
  executablePath: executable,
  env,
  args: ['--no-sandbox', '--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan'],
});
const page = await browser.newPage({ viewport: { width: 640, height: 480 } });
page.on('console', (m) => console.log(`console ${m.type()}: ${m.text()}`));
page.on('pageerror', (e) => console.log(`pageerror: ${e.message}`));
await page.goto(url);
await page.waitForTimeout(4000);
await page.screenshot({ path: `${out}-1.png` });
await page.keyboard.press('ArrowLeft');
await page.keyboard.press('ArrowUp');
await page.waitForTimeout(500);
await page.screenshot({ path: `${out}-2.png` });
const error = (await page.textContent('#error')) ?? '';
await browser.close();
if (error.trim()) {
  console.log(`#error: ${error}`);
  process.exit(1);
}
console.log(`saved ${out}-1.png and ${out}-2.png; #error is empty`);
