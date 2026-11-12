// Camera switching must animate independently of driving, then stop drawing when paused.
import assert from 'node:assert/strict';
import {mkdir} from 'node:fs/promises';
const {PLAYWRIGHT_MODULE,RACER_URL,RACER_EVIDENCE}=process.env;
if(!PLAYWRIGHT_MODULE||!RACER_URL||!RACER_EVIDENCE) throw new Error('Set PLAYWRIGHT_MODULE, RACER_URL, RACER_EVIDENCE');
const {chromium}=await import(PLAYWRIGHT_MODULE);
await mkdir(RACER_EVIDENCE,{recursive:true});
const env={...process.env,VK_DRIVER_FILES:'/usr/lib/chromium/vk_swiftshader_icd.json'};delete env.DBUS_SESSION_BUS_ADDRESS;
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,env,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-webgpu','--enable-features=Vulkan,WebGPU']});
const errors=[];
async function state(page){return page.locator('#status').evaluate(e=>JSON.parse(e.textContent));}
async function click(page,name){const s=await state(page);const [x,y,w,h]=s.targets[name];await page.mouse.click(x+w/2,y+h/2);}
try {
 for(const [width,height] of [[1100,720],[320,568],[740,250]]) {
  const page=await browser.newPage({viewport:{width,height},hasTouch:true});
  // Camera settling takes about 4 seconds; software GPU startup previously <3 s.
  page.setDefaultTimeout(20000);page.on('pageerror',e=>errors.push(String(e)));
  await page.goto(RACER_URL+'?demo=racer&inspect=1');
  await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('{'));
  await click(page,'camera');
  await page.waitForFunction(()=>{const s=JSON.parse(document.querySelector('#status').textContent);return s.cameraFollow&&!s.cameraMoving;});
  const follow=await state(page);assert(follow.paused);assert(follow.zoom>1.2);assert(follow.cameraTilt<0.7);assert(Math.abs(follow.cameraYaw-Math.PI/2)<0.01);
  for(const [name,[x,y,w,h]] of Object.entries(follow.targets))assert(x>=0&&y>=0&&x+w<=width+1&&y+h<=height+1,`${name} outside ${width}x${height}`);
  await page.waitForTimeout(350);assert.equal((await state(page)).frame,follow.frame);
  await page.screenshot({path:`${RACER_EVIDENCE}/follow-${width}.png`});
  if(width===1100){
   await click(page,'toggle');await page.locator('canvas').focus();await page.keyboard.down('ArrowUp');await page.keyboard.down('ArrowRight');
   await page.waitForFunction(()=>JSON.parse(document.querySelector('#status').textContent).heading>0.3);
   const turning=await state(page);assert(turning.cameraYaw>Math.PI/2);assert(!turning.paused);
   await page.keyboard.up('ArrowRight');await page.keyboard.up('ArrowUp');
   await click(page,'camera');assert(!(await state(page)).paused);
   await page.waitForFunction(()=>Math.abs(JSON.parse(document.querySelector('#status').textContent).cameraYaw)<0.005);
   await click(page,'toggle');
  }else{await click(page,'camera');}
  await page.waitForFunction(()=>{const s=JSON.parse(document.querySelector('#status').textContent);return !s.cameraFollow&&!s.cameraMoving;});
  const fixed=await state(page);assert(fixed.paused);assert(Math.abs(fixed.cameraYaw)<0.01);
  await page.screenshot({path:`${RACER_EVIDENCE}/fixed-${width}.png`});
  assert.equal(await page.locator('#error').innerText(),'');await page.close();
 }
 assert.deepEqual(errors,[]);console.log('Fixed/Follow transitions, moving switch, pause-idle and phone bounds passed');
}finally{await browser.close();}
