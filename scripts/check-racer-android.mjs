// Use a matching physical display scale: Chromium's deviceScaleFactor alone
// disagrees with ResizeObserver device-pixel measurements.
import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
if(!process.env.PLAYWRIGHT_MODULE) throw new Error('Set PLAYWRIGHT_MODULE to playwright-core.');
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE);
const base=process.env.RACER_URL||'https://debian.tailf56695.ts.net:8443/racer/';
const output=process.env.RACER_EVIDENCE||'/tmp/racer-android';await mkdir(output,{recursive:true});
const env={...process.env,VK_DRIVER_FILES:'/usr/lib/chromium/vk_swiftshader_icd.json'};delete env.DBUS_SESSION_BUS_ADDRESS;
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,env,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-webgpu','--enable-features=Vulkan,WebGPU','--force-device-scale-factor=3']});
const records=[];
try {
 const page=await browser.newPage({viewport:{width:390,height:844},deviceScaleFactor:3,isMobile:true,hasTouch:true});page.setDefaultTimeout(20000);
 await page.goto(base+'?demo=racer&inspect=1');
 await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('{'));
 const state=()=>page.locator('#status').evaluate(e=>JSON.parse(e.textContent));
 // Model the separate visual-viewport height reported as Android's browser bars appear.
 for(const height of [650,780,600]) {
  await page.evaluate(h=>{Object.defineProperty(visualViewport,'height',{configurable:true,get:()=>h});visualViewport.dispatchEvent(new Event('resize'));},height);
  await page.waitForFunction(h=>Math.abs(document.querySelector('canvas').getBoundingClientRect().height-h)<1,height,{timeout:2000});
  await page.waitForFunction(h=>{const s=JSON.parse(document.querySelector('#status').textContent);return Object.values(s.targets).every(([x,y,w,height])=>y>=0&&y+height<=h+1);},height);
  const s=await state();records.push({visualHeight:height,state:s});
  await page.screenshot({path:`${output}/viewport-${height}.png`});
 }
 const s=await state();const [x,y,w,h]=s.targets.gas;const cdp=await page.context().newCDPSession(page);
 await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{id:1,x:x+w/2,y:y+h/2}]});
 await page.waitForFunction(()=>JSON.parse(document.querySelector('#status').textContent).speed>10);
 // A real main-thread stall reproduces the previous >250ms pause path.
 await page.evaluate(()=>{const end=performance.now()+400;while(performance.now()<end){};});
 await page.waitForTimeout(150);const afterStall=await state();assert.equal(afterStall.paused,false);assert.equal(afterStall.throttle,1);
 await cdp.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
 assert.equal(await page.locator('#error').innerText(),'');records.push({afterStall});
 await writeFile(`${output}/result.json`,JSON.stringify({base,records},null,2));console.log('Android visual viewport and slow-frame checks passed');
} catch(error){await writeFile(`${output}/failure.json`,JSON.stringify({error:String(error),records},null,2));throw error;}
finally {await browser.close();}
