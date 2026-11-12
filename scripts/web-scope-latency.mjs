// popErrorScope latency in headless Chromium WebGPU: the time from
// popErrorScope to its promise resolving, once per animation frame, around
// a render pass and submit as a gpu-surface's render makes, and around a
// call that makes a validation error. Prints the distribution as JSON.
// Sets the scope deadline in crates/app/src/painter_surface_scope.rs.
//
//   PLAYWRIGHT_MODULE=~/projects/game/node_modules/playwright-core/index.mjs \
//     node scripts/web-scope-latency.mjs [renders]
import http from 'node:http';
const {PLAYWRIGHT_MODULE}=process.env;
if(!PLAYWRIGHT_MODULE) throw new Error('Set PLAYWRIGHT_MODULE to playwright-core/index.mjs (the file); rerun: PLAYWRIGHT_MODULE=… node scripts/web-scope-latency.mjs');
const renders=Number(process.argv[2]??500);
const {chromium}=await import(PLAYWRIGHT_MODULE);
const page=`<!doctype html><script type="module">
const adapter=await navigator.gpu.requestAdapter();const device=await adapter.requestDevice();
const texture=device.createTexture({size:[512,512],format:'rgba8unorm',usage:GPUTextureUsage.RENDER_ATTACHMENT|GPUTextureUsage.TEXTURE_BINDING});
const module=device.createShaderModule({code:'@vertex fn v(@builtin(vertex_index) i:u32)->@builtin(position) vec4f{let p=array(vec2f(-1,-1),vec2f(3,-1),vec2f(-1,3));return vec4f(p[i],0,1);} @fragment fn f(@builtin(position) p:vec4f)->@location(0) vec4f{return vec4f(p.x/512,p.y/512,0.5,1);}'});
const pipeline=device.createRenderPipeline({layout:'auto',vertex:{module,entryPoint:'v'},fragment:{module,entryPoint:'f',targets:[{format:'rgba8unorm'}]}});
function render(){const encoder=device.createCommandEncoder();const pass=encoder.beginRenderPass({colorAttachments:[{view:texture.createView(),loadOp:'clear',storeOp:'store'}]});pass.setPipeline(pipeline);pass.draw(3);pass.end();device.queue.submit([encoder.finish()]);}
function invalid(){device.createBuffer({size:4,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.MAP_WRITE});}
const frame=()=>new Promise(r=>requestAnimationFrame(r));
async function scoped(call){device.pushErrorScope('validation');call();const start=performance.now();const error=await device.popErrorScope();return {ms:performance.now()-start,error:!!error};}
window.measure=async(n)=>{const out={render:[],invalid:[]};for(let i=0;i<n;i++){await frame();out.render.push(await scoped(render));}for(let i=0;i<n/10;i++){await frame();out.invalid.push(await scoped(invalid));}return out;};
window.ready=true;
</script>`;
const server=http.createServer((_,res)=>{res.setHeader('content-type','text/html');res.end(page);});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const env={...process.env,VK_DRIVER_FILES:'/usr/lib/chromium/vk_swiftshader_icd.json'};
delete env.DBUS_SESSION_BUS_ADDRESS;
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,env,args:['--no-sandbox','--disable-dev-shm-usage','--use-angle=swiftshader','--enable-unsafe-webgpu','--enable-features=Vulkan,WebGPU']});
function summary(samples){
 const ms=samples.map(s=>s.ms).sort((a,b)=>a-b);const at=q=>+ms[Math.min(ms.length-1,Math.floor(q*ms.length))].toFixed(2);
 return {n:ms.length,errors:samples.filter(s=>s.error).length,p50:at(0.5),p90:at(0.9),p99:at(0.99),max:+ms.at(-1).toFixed(2)};
}
try {
 const tab=await browser.newPage();tab.setDefaultTimeout(120000);
 tab.on('pageerror',e=>{throw e;});
 await tab.goto(`http://127.0.0.1:${server.address().port}/`);
 // SwiftShader WebGPU startup measured under 3 s; 30 s on a loaded machine.
 await tab.waitForFunction(()=>window.ready,null,{timeout:30000});
 const out=await tab.evaluate(n=>window.measure(n),renders);
 console.log(JSON.stringify({chromium:browser.version(),render:summary(out.render),invalid:summary(out.invalid)}));
} finally {await browser.close();server.close();}
