const fs=require('node:fs/promises'),path=require('node:path'),assert=require('node:assert/strict'),net=require('node:net'),{spawn}=require('node:child_process'),{connect}=require('./tauri-check.cjs');
const root=__dirname,delay=ms=>new Promise(r=>setTimeout(r,ms));
async function main(){
 const server=net.createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const port=server.address().port;await new Promise(r=>server.close(r));
 const profile=path.join(root,'qa-startup-browser-'+Date.now());
 const child=spawn('C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',['--headless=new','--no-first-run','--no-default-browser-check','--enable-unsafe-swiftshader','--remote-debugging-port='+port,'--user-data-dir='+profile,'--window-size=1440,940','about:blank'],{windowsHide:true,stdio:'ignore'});
 let cdp;
 try{
  cdp=await connect(port);await cdp.send('Page.enable');
  const reload=async()=>{const origin=await cdp.eval('performance.timeOrigin');await cdp.send('Page.reload');await cdp.until(`performance.timeOrigin!==${origin}&&!!document.querySelector('.launcher-shell')`)};
  const instrumentation=`window.__bootQA={frames:[],contexts:0};const raf=window.requestAnimationFrame;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__bootQA.frames.push(t);cb(t)});const get=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...args){const ctx=get.call(this,type,...args);if(type==='webgl2'&&ctx){window.__bootGL=ctx;window.__bootQA.contexts++}return ctx};window.launcher={engine:async m=>m==='icons.list'?{icons:[]}:{accounts:[],jobs:[]},library:async()=>[{id:'test',name:'Minecraft 1.20.1',version:'1.20.1',loader:'Fabric',icon:'graphite-v1-categories-02',playtime:0,lastPlayed:0}],versions:async()=>[{version:'1.20.1',type:'release',date:'2023-06-12'}],onError:async()=>()=>{},onFileDrop:async()=>()=>{},window:async()=>{}};`;
  await cdp.send('Page.addScriptToEvaluateOnNewDocument',{source:instrumentation});
  await cdp.send('Page.addScriptToEvaluateOnNewDocument',{source:"window.launcher.tools=async()=>null"});
  const shot=async name=>fs.writeFile(path.join(root,'../outputs/Graphite-startup-'+name+'.png'),Buffer.from((await cdp.send('Page.captureScreenshot',{format:'png'})).data,'base64'));
  await cdp.send('Page.navigate',{url:'http://127.0.0.1:5187'});
  await cdp.until("document.querySelector('.startup')?.dataset.mode==='webgl'");
  assert(await cdp.eval("document.querySelector('.launcher-shell').inert"));
  await delay(120);await shot('01-closeup');await delay(750);await shot('02-assembly');await delay(650);await shot('03-logo');
  await cdp.until("document.querySelector('.startup')?.dataset.phase==='exit'");await delay(220);assert(await cdp.eval("Number(getComputedStyle(document.querySelector('.startup-caption')).opacity)<.1"),'Wordmark did not fade during handoff');
  await cdp.until("!document.querySelector('.startup')");assert.equal(await cdp.eval("document.querySelector('.launcher-shell').inert"),false);
  await delay(150);assert(await cdp.eval("window.__bootGL.isContextLost()"),'WebGL context was not released');
  const frames=await cdp.eval("window.__bootQA.frames");const intervals=frames.slice(1).map((t,i)=>t-frames[i]).filter(n=>n>0).sort((a,b)=>a-b);console.log('Normal startup frames:',frames.length,'median interval:',intervals[Math.floor(intervals.length*.5)]?.toFixed(1),'p95:',intervals[Math.floor(intervals.length*.95)]?.toFixed(1),'ms');
  const count=frames.length;await delay(300);assert.equal(await cdp.eval("window.__bootQA.frames.length"),count,'Animation continues after exit');await shot('04-workspace');
  await reload();await cdp.until("!!document.querySelector('.startup-footer button')");await cdp.send('Input.dispatchKeyEvent',{type:'keyDown',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});await cdp.send('Input.dispatchKeyEvent',{type:'keyUp',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});await cdp.until("!document.querySelector('.startup')");assert.equal(await cdp.eval("document.querySelector('.launcher-shell').inert"),false);
  await cdp.send('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});await reload();await cdp.until("!!document.querySelector('.launcher-revealed')");assert.equal(await cdp.eval("window.__bootQA.contexts"),0,'Reduced motion created WebGL');
  await cdp.send('Emulation.setEmulatedMedia',{features:[]});await cdp.send('Page.addScriptToEvaluateOnNewDocument',{source:"const oldContext=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...args){return type==='webgl2'?null:oldContext.call(this,type,...args)}"});await reload();await cdp.until("!!document.querySelector('.launcher-revealed')");assert.equal(await cdp.eval("document.querySelector('.launcher-shell').inert"),false);
  console.log('PASS normal 3D assembly, handoff, GPU disposal/no continued RAF, immediate skip, reduced-motion without WebGL, unavailable-WebGL fallback.');
 }finally{if(cdp){await cdp.send('Browser.close').catch(()=>{});cdp.ws.close()}await Promise.race([new Promise(r=>child.once('exit',r)),delay(5000)]);if(child.exitCode===null)child.kill();}
}
main().catch(e=>{console.error(e.stack);process.exitCode=1});
