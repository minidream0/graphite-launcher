const fs=require('node:fs/promises'),path=require('node:path'),net=require('node:net'),assert=require('node:assert/strict'),{spawn}=require('node:child_process'),{connect}=require('./tauri-check.cjs');
const root=__dirname,delay=ms=>new Promise(r=>setTimeout(r,ms)),version=require('./graphite-web/package.json').version;
const binary=process.env.GRAPHITE_PORTHOLE_TEST||'C:/Program Files (x86)/Steam/steamapps/common/porthole/porthole.exe';
async function waitFor(read){for(let n=0;n<250;n++){const result=await read();if(result)return result;await delay(100)}throw Error('Timed out')}
async function main(){
 const dir=path.join(root,'qa-porthole-'+Date.now()),data=path.join(dir,'data');await fs.mkdir(path.join(data,'instances/test/minecraft/saves/protected'),{recursive:true});
 await fs.writeFile(path.join(data,'instances/test/instance.cfg'),'[General]\nname=Co-op QA\nInstanceType=OneSix\n');
 await fs.writeFile(path.join(data,'instances/test/mmc-pack.json'),JSON.stringify({formatVersion:1,components:[{uid:'net.minecraft',version:'1.20.1'}]}));
 await fs.writeFile(path.join(data,'instances/test/minecraft/saves/protected/level.dat'),'keep-world');
 await fs.writeFile(path.join(data,'graphite-seen-version.json'),JSON.stringify(version));
 const sockets=new Set(),server=net.createServer(socket=>{sockets.add(socket);socket.on('close',()=>sockets.delete(socket));socket.on('error',()=>{});socket.on('data',data=>socket.write(data))});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));const gamePort=server.address().port;
 let child,cdp,host;const events=[];let errors='';
 const stop=async process=>{if(process&&process.exitCode===null){process.stdin?.write('quit\n');await Promise.race([new Promise(r=>process.once('exit',r)),delay(2000)]);if(process.exitCode===null){process.kill();await new Promise(r=>process.once('exit',r))}}};
 const shot=async name=>{await cdp.eval('document.fonts.ready');await fs.writeFile(path.join(root,'../outputs/Graphite-porthole-'+name+'.png'),Buffer.from((await cdp.send('Page.captureScreenshot',{format:'png'})).data,'base64'))};
 try{
  const portSocket=net.createServer();await new Promise(r=>portSocket.listen(0,'127.0.0.1',r));const debugPort=portSocket.address().port;await new Promise(r=>portSocket.close(r));
  child=spawn(path.join(root,'graphite-web/src-tauri/target/release/graphite.exe'),[],{env:{...process.env,GRAPHITE_DATA:data,GRAPHITE_BACKEND:process.env.GRAPHITE_QA_ENGINE||path.join(root,'../outputs/Graphite/engine/prismlauncher.exe'),WEBVIEW2_USER_DATA_FOLDER:path.join(dir,'webview'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port='+debugPort},windowsHide:true,stdio:'ignore'});
  cdp=await connect(debugPort);await cdp.send('Page.enable');await cdp.until("document.querySelectorAll('.library-item').length===1&&!document.querySelector('.startup')");
  const click=label=>cdp.eval(`(()=>{const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===${JSON.stringify(label)});if(!b||b.disabled)throw Error('Button unavailable: '+${JSON.stringify(label)});b.click()})()`);
  const input=(label,value)=>cdp.eval(`(()=>{const e=document.querySelector('[aria-label='+${JSON.stringify(JSON.stringify(label))}+']');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,${JSON.stringify(value)});e.dispatchEvent(new Event('input',{bubbles:true}))})()`);
  await click('Серверы');await cdp.until("!!document.querySelector('.server-tabs')");await click('С друзьями');await cdp.until("!!document.querySelector('.porthole-form')");
  assert.equal((await cdp.eval("window.launcher.porthole('status')")).installed,true);
  await click('Открыть комнату');await cdp.until("document.querySelector('.porthole [role=alert]')?.textContent.includes('порт')");assert(await cdp.eval("!!document.querySelector('.porthole-form')"));
  await input('Порт мира Porthole','65536');await click('Открыть комнату');assert(await cdp.eval("!!document.querySelector('.porthole-form')"));
  await input('Порт мира Porthole',String(gamePort));await shot('graphite-form');await click('Открыть комнату');
  await cdp.until("document.querySelector('.porthole-code strong')?.textContent.length>5");
  await cdp.until("window.launcher.porthole('status').then(s=>s.phase==='hosting')");
  let status=await cdp.eval("window.launcher.porthole('status')");assert.equal(status.phase,'hosting');assert(status.steamId);assert.equal(status.port,gamePort);
  const hostCode=status.code;await click('Копировать');await cdp.until("!!document.querySelector('[aria-label=\"Скопировать код комнаты\"] svg')");
  await click('Избранные серверы');await cdp.until("!!document.querySelector('.server-list')");await click('С друзьями');await cdp.until("!!document.querySelector('.porthole-code')");assert.equal((await cdp.eval("window.launcher.porthole('status')")).code,hostCode);
  await shot('graphite-host');await click('Закрыть комнату');await cdp.until("!!document.querySelector('.porthole-form')");
  console.log('PASS real Steam host lobby, code, port, copy, tab reopen and stop; validation stays visible');
  host=spawn(binary,['--json','--anonymous','expose','tcp/127.0.0.1:'+gamePort,'--game-name','Graphite QA'],{cwd:path.dirname(binary),windowsHide:true,stdio:['pipe','pipe','pipe']});
  let buffer='';host.stdout.on('data',chunk=>{buffer+=chunk;let pos;while((pos=buffer.indexOf('\n'))>=0){const line=buffer.slice(0,pos);buffer=buffer.slice(pos+1);try{events.push(JSON.parse(line))}catch{}}});host.stderr.on('data',data=>{errors=(errors+data).slice(-4000)});
  const event=await waitFor(()=>events.find(e=>e.event==='server_ready'));
  await click('К другу');await input('Код комнаты Porthole',event.code);await input('Порт мира Porthole',String(gamePort));await click('Подключиться к комнате');
  await cdp.until("document.querySelector('.porthole-connection')?.textContent==='Соединение установлено'");
  status=await cdp.eval("window.launcher.porthole('status')");assert.match(status.address,/^127\.0\.0\.1:\d+$/);
  const [address,localPort]=status.address.split(':');const result=await new Promise((resolve,reject)=>{const socket=net.connect(Number(localPort),address);socket.setTimeout(8000,()=>socket.destroy(Error('Echo timeout')));socket.on('error',reject);socket.once('connect',()=>socket.write('graphite-real-steam-tunnel'));socket.once('data',data=>{socket.end();resolve(data.toString())})});assert.equal(result,'graphite-real-steam-tunnel');
  await shot('graphite-joined');await cdp.eval("document.documentElement.dataset.theme='smoky'");await shot('smoky-joined');
  await cdp.send('Emulation.setDeviceMetricsOverride',{width:900,height:660,deviceScaleFactor:1,mobile:false});
  assert(await cdp.eval("document.documentElement.scrollWidth<=innerWidth"),'Horizontal overflow in compact window');
  await shot('compact');await cdp.send('Emulation.clearDeviceMetricsOverride');
  assert.equal(await cdp.eval("document.querySelector('.porthole-session .primary-button').disabled"),false);
  await stop(host);host=null;
  await cdp.until("window.launcher.porthole('status').then(s=>s.phase!=='connected')");
  assert.notEqual((await cdp.eval("window.launcher.porthole('status')")).phase,'connected');
  await click('Отключиться');await cdp.until("!!document.querySelector('.porthole-form')");assert.equal((await cdp.eval("window.launcher.porthole('status')")).active,false);
  assert.equal(await fs.readFile(path.join(data,'instances/test/minecraft/saves/protected/level.dat'),'utf8'),'keep-world');
  assert.equal((await cdp.eval("window.launcher.engine('state')")).visibleWindows,0);
  console.log('PASS real Steam-to-anonymous Porthole tunnel, actual TCP echo, process-owned loopback address, both themes and preserved worlds');
  console.log('Fixture:',dir);
 }finally{
  if(cdp){await cdp.eval("window.launcher.porthole('stop')").catch(()=>{});await cdp.eval("window.launcher.window('close')").catch(()=>{});cdp.ws.close()}
  await stop(host);if(child&&child.exitCode===null)await Promise.race([new Promise(r=>child.once('exit',r)),delay(8000)]);if(child&&child.exitCode===null)child.kill();
  for(const socket of sockets)socket.destroy();await new Promise(r=>server.close(r));
 }
}
main().catch(e=>{console.error(e.stack);process.exitCode=1});
