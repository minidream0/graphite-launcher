import {operations} from './operations';
if(!window.launcher&&window.__TAURI__){
 const tauri=window.__TAURI__;
 const call=(channel,...args)=>operations.observe(channel,args,()=>tauri.core.invoke('rpc',{channel,args:args.map(value=>value??null)}).catch(error=>{throw new Error(typeof error==='string'?error:String(error))}));
 const on=(name,callback)=>tauri.event.listen(name,event=>callback(event.payload));
 window.launcher=Object.freeze({
  porthole:(...args)=>call('porthole',...args),
  tools:(...args)=>call('tools',...args),queue:(...args)=>call('queue',...args),
  library:()=>call('library'),versions:(...args)=>call('versions',...args),engine:(...args)=>call('engine',...args),modrinth:(...args)=>call('modrinth',...args),resources:(...args)=>call('resources',...args),servers:(...args)=>call('servers',...args),packUpdate:(...args)=>call('pack-update',...args),installedMods:(...args)=>call('installed-mods',...args),copyLog:(...args)=>call('copy-log',...args),backups:(...args)=>call('backups',...args),pickImport:()=>call('pick-import'),dropFiles:(...args)=>call('drop-files',...args),importInstance:(...args)=>call('import-instance',...args),chooseJava:()=>call('choose-java'),loginAction:(...args)=>call('login-action',...args),twitchland:(...args)=>call('twitchland',...args),action:(...args)=>call('action',...args),window:action=>call('window',action),onFileDrop:callback=>on('graphite-file-drag',callback),onError:callback=>on('graphite-error',callback)
 });
}
