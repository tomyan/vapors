// Run against the built demo. Requires PLAYWRIGHT_MODULE, RACER_URL and RACER_EVIDENCE.
import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
const {PLAYWRIGHT_MODULE,RACER_URL,RACER_EVIDENCE}=process.env;
if(!PLAYWRIGHT_MODULE||!RACER_URL||!RACER_EVIDENCE) throw new Error('Set PLAYWRIGHT_MODULE to playwright-core, RACER_URL to the demo server, and RACER_EVIDENCE to an output directory.');
const {chromium}=await import(PLAYWRIGHT_MODULE);
await mkdir(RACER_EVIDENCE,{recursive:true});
const env={...process.env,VK_DRIVER_FILES:'/usr/lib/chromium/vk_swiftshader_icd.json'};
delete env.DBUS_SESSION_BUS_ADDRESS;
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,env,args:['--no-sandbox','--disable-dev-shm-usage','--use-angle=swiftshader','--enable-unsafe-webgpu','--enable-features=Vulkan,WebGPU']});
const errors=[];const records=[];
// First software-WebGPU startup measured under 3 seconds; allow 20 seconds on a loaded machine.
const deadline=20000;
async function state(page){return page.locator('#status').evaluate(el=>JSON.parse(el.textContent));}
async function until(page,expression){await page.waitForFunction(expression,null,{timeout:deadline});}
async function open(viewport){
 const page=await browser.newPage({viewport,hasTouch:true});page.setDefaultTimeout(deadline);
 page.on('pageerror',e=>errors.push(String(e)));
 await page.goto(RACER_URL+'?demo=racer&inspect=1');
 await until(page,()=>document.querySelector('#error').textContent||document.querySelector('#status').textContent.startsWith('{'));
 assert.equal(await page.locator('#error').innerText(),'');return page;
}
async function click(page,name){const s=await state(page);const [x,y,w,h]=s.targets[name];await page.mouse.click(x+w/2,y+h/2);}
async function capture(page,name){await page.screenshot({path:`${RACER_EVIDENCE}/${name}.png`});}
try {
 const page=await open({width:1100,height:720});
 await capture(page,'desktop-start');
 const idle=await state(page);await page.waitForTimeout(350);assert.equal((await state(page)).frame,idle.frame);
 await click(page,'toggle');await page.locator('canvas').focus();await page.keyboard.down('ArrowUp');
 await until(page,()=>JSON.parse(document.querySelector('#status').textContent).speed>100);
 const moving=await state(page);assert(moving.zoom<0.96);assert(moving.x>idle.x+20);
 await capture(page,'desktop-speed');
 await page.keyboard.up('ArrowUp');await page.keyboard.down('ArrowRight');
 await until(page,()=>JSON.parse(document.querySelector('#status').textContent).heading>0.15);
 await page.keyboard.up('ArrowRight');await page.keyboard.press('Space');
 await until(page,()=>JSON.parse(document.querySelector('#status').textContent).paused);
 await page.waitForTimeout(100);const paused=await state(page);await page.waitForTimeout(350);
 assert.equal((await state(page)).frame,paused.frame);assert.equal((await state(page)).time,paused.time);
 records.push({case:'drive-camera-steer-pause-idle',moving,paused});
 await click(page,'restart');await until(page,()=>JSON.parse(document.querySelector('#status').textContent).time===0);
 assert.equal((await state(page)).zoom,1);
 await click(page,'tune');await until(page,()=>!!JSON.parse(document.querySelector('#status').textContent).targets.steering);
 await click(page,'steering');await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');await page.keyboard.type('1.25');await page.keyboard.press('Enter');
 await until(page,()=>JSON.parse(document.querySelector('#status').textContent).steering===1.25);
 await capture(page,'desktop-tuning');records.push({case:'typed-tuning',state:await state(page)});
 await click(page,'toggle');await page.locator('canvas').focus();await page.keyboard.down('KeyW');
 await until(page,()=>JSON.parse(document.querySelector('#status').textContent).speed>15);
 await page.evaluate(()=>{const button=document.createElement('button');button.id='focus-test';button.textContent='Focus test';document.body.append(button);button.focus();});
 await until(page,()=>JSON.parse(document.querySelector('#status').textContent).paused);
 assert.equal((await state(page)).throttle,0);await page.keyboard.up('KeyW');
 records.push({case:'blur-pauses-clears-held-input',state:await state(page)});await page.close();
 for(const width of [390,320]){
  const phone=await open({width,height:700});await capture(phone,`phone-${width}`);
  const s=await state(phone);
  for(const name of ['left','right','gas','brake','toggle','restart','tune']){
   const [x,y,w,h]=s.targets[name];assert(w>=44&&h>=44);assert(x>=0&&x+w<=width&&y>=0&&y+h<=700);
  }
  const point=(name,id)=>{const [x,y,w,h]=s.targets[name];return {x:x+w/2,y:y+h/2,id};};
  const cdp=await phone.context().newCDPSession(phone);
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[point('gas',1)]});
  await until(phone,()=>JSON.parse(document.querySelector('#status').textContent).speed>10);
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[point('gas',1),point('right',2)]});
  await until(phone,()=>JSON.parse(document.querySelector('#status').textContent).steer===1);
  await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[point('right',2)]});
  await until(phone,()=>{const s=JSON.parse(document.querySelector('#status').textContent);return s.steer===0&&s.throttle===1;});
  await cdp.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
  await until(phone,()=>JSON.parse(document.querySelector('#status').textContent).throttle===0);
  await click(phone,'toggle');await capture(phone,`phone-${width}-driven`);
  records.push({case:`touch-${width}`,state:await state(phone)});assert.equal(await phone.locator('#error').innerText(),'');await phone.close();
 }
 assert.deepEqual(errors,[]);
 await writeFile(`${RACER_EVIDENCE}/browser.json`,JSON.stringify({records,errors},null,2));
 console.log(JSON.stringify({passed:records.map(r=>r.case),errors}));
} catch(error) {
 const pages=await Promise.all(browser.contexts().flatMap(c=>c.pages()).map(async p=>({url:p.url(),status:await p.locator('#status').textContent().catch(()=>null),error:await p.locator('#error').textContent().catch(()=>null)})));await writeFile(`${RACER_EVIDENCE}/browser-failure.json`,JSON.stringify({error:String(error),stack:error.stack,records,errors,pages},null,2));throw error;
} finally {await browser.close();}
