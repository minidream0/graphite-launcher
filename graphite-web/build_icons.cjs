// Render the vector master with Edge and package native PNG frames into one Windows ICO.
const fs=require('node:fs/promises'),path=require('node:path'),net=require('node:net'),assert=require('node:assert/strict'),{spawn}=require('node:child_process');
const {connect}=require('../tauri-check.cjs');
async function main(){
 const root=__dirname,out=path.join(root,'src-tauri/icons'),preview=path.resolve(root,'../../outputs/logo-06');await fs.mkdir(out,{recursive:true});await fs.mkdir(preview,{recursive:true});
 const browser=process.env.GRAPHITE_EDGE||'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';await fs.access(browser);
 const listener=net.createServer();await new Promise(r=>listener.listen(0,'127.0.0.1',r));const port=listener.address().port;await new Promise(r=>listener.close(r));
 const user=await fs.mkdtemp(path.join(require('node:os').tmpdir(),'graphite-logo-'));
 const child=spawn(browser,['--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check',`--user-data-dir=${user}`,`--remote-debugging-port=${port}`,'about:blank'],{windowsHide:true,stdio:'ignore'});let cdp;
 try{
  cdp=await connect(port);const svg=await fs.readFile(path.join(root,'public/graphite.svg'),'utf8');
  await cdp.eval(`document.documentElement.innerHTML='<head><style>html,body{margin:0;background:transparent;overflow:hidden}svg{display:block;width:100vw;height:100vh}</style></head><body>'+${JSON.stringify(svg)}+'</body>'`);
  await cdp.send('Emulation.setDefaultBackgroundColorOverride',{color:{r:0,g:0,b:0,a:0}});
  const sizes=[16,20,24,32,40,48,64,128,256],frames=[];
  for(const size of sizes){await cdp.send('Emulation.setDeviceMetricsOverride',{width:size,height:size,deviceScaleFactor:1,mobile:false});const png=Buffer.from((await cdp.send('Page.captureScreenshot',{format:'png',clip:{x:0,y:0,width:size,height:size,scale:1}})).data,'base64');assert.equal(png.readUInt32BE(16),size);assert.equal(png.readUInt32BE(20),size);frames.push(png);await fs.writeFile(path.join(preview,`Graphite-${size}.png`),png);if(size===32)await fs.writeFile(path.join(out,'32x32.png'),png);if(size===128)await fs.writeFile(path.join(out,'128x128.png'),png);if(size===256)await fs.writeFile(path.join(out,'icon.png'),png);}
  const header=Buffer.alloc(6+16*sizes.length);header.writeUInt16LE(1,2);header.writeUInt16LE(sizes.length,4);let offset=header.length;
  frames.forEach((png,i)=>{const entry=6+16*i;header[entry]=sizes[i]%256;header[entry+1]=sizes[i]%256;header.writeUInt16LE(1,entry+4);header.writeUInt16LE(32,entry+6);header.writeUInt32LE(png.length,entry+8);header.writeUInt32LE(offset,entry+12);offset+=png.length;});
  const ico=Buffer.concat([header,...frames]);assert.equal(ico.length,offset);await fs.writeFile(path.join(out,'icon.ico'),ico);await fs.writeFile(path.join(preview,'Graphite.ico'),ico);await fs.copyFile(path.join(root,'public/graphite.svg'),path.join(preview,'Graphite.svg'));
  console.log('PASS vector logo rendered at 16/20/24/32/40/48/64/128/256px; transparent PNG frames and multi-size ICO');
 }finally{if(cdp){await cdp.send('Browser.close').catch(()=>{});cdp.ws.close();}else child.kill();}
}
main().catch(e=>{console.error(e);process.exitCode=1});
