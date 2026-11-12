// Verify the compiled HUD in short phone viewports, including browser chrome.
import assert from 'node:assert/strict';
import {mkdir} from 'node:fs/promises';
const {chromium} = await import(process.env.PLAYWRIGHT_MODULE);
const output = process.env.RACER_EVIDENCE;
if (!output || !process.env.RACER_URL) throw new Error('Set RACER_URL and RACER_EVIDENCE');
await mkdir(output, {recursive:true});
const env = {...process.env, VK_DRIVER_FILES:'/usr/lib/chromium/vk_swiftshader_icd.json'};
delete env.DBUS_SESSION_BUS_ADDRESS;
const browser = await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,env,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-webgpu','--enable-features=Vulkan,WebGPU']});
try {
  for (const [width,height] of [[740,250],[844,300],[640,260]]) {
    const page = await browser.newPage({viewport:{width,height},hasTouch:true});
    // First software GPU startup measured under 3 s; allow 20 s under load.
    page.setDefaultTimeout(20000);
    const errors=[]; page.on('pageerror', error => errors.push(String(error)));
    await page.goto(process.env.RACER_URL+'?demo=racer&inspect=1');
    await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('{'));
    const read=()=>page.locator('#status').evaluate(node=>JSON.parse(node.textContent));
    const paused=await read();
    for (const [name,[x,y,w,h]] of Object.entries(paused.targets)) {
      assert(x>=0 && y>=0 && x+w<=width+1 && y+h<=height+1, `${name} outside ${width}x${height}`);
    }
    await page.screenshot({path:`${output}/${width}-paused.png`});
    const [x,y,w,h]=paused.targets.gas;
    await page.mouse.move(x+w/2,y+h/2); await page.mouse.down();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('#status').textContent).speed>20);
    await page.screenshot({path:`${output}/${width}-driving.png`});
    await page.mouse.up();
    assert.equal(await page.locator('#error').innerText(),'');
    assert.deepEqual(errors,[]);
    await page.close();
  }
  console.log('Short landscape controls and GPU rendering passed');
} finally {await browser.close();}
