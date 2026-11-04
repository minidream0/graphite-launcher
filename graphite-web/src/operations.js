// Only presentation state lives here; hiding a scene never cancels its task.
export function createOperations(){
 let sequence=0,snapshot=[],listeners=new Set();
 const publish=()=>{snapshot=snapshot.slice(-64);for(const fn of listeners)fn()};
 const update=(id,patch)=>{snapshot=snapshot.map(item=>item.id===id?{...item,...patch}:item);publish()};
 function begin(kind,title){const id=++sequence;snapshot=[...snapshot,{id,kind,title,state:'pending',status:'Подготовка…',started:Date.now()}];publish();return id}
 function finish(id,state,status){update(id,{state,status,ended:Date.now()})}
 async function observe(channel,args,request){
  const [method,p={}]=args;
  const retried=channel==='queue'&&method==='retry'?snapshot.find(item=>item.job===p):null;
  let kind=retried?.kind||null;
  if(channel==='engine'&&method==='launch')kind='portal';
  if(channel==='servers'&&method==='launch')kind='route';
  if(channel==='tools'&&method==='preflight')kind='scan';
  if(channel==='engine'&&method==='create'||channel==='import-instance'||channel==='servers'&&method==='prepare'||channel==='pack-update'&&method==='update')kind='assembly';
  if(channel==='modrinth'&&method==='install'&&['mod','modpack'].includes(p?.kind))kind=p.kind==='mod'?'mods':'assembly';
  const titles={portal:'Запуск Minecraft',route:'Подготовка подключения',scan:'Проверка сборки',assembly:'Установка сборки',mods:'Установка модов'};
  let id=kind?begin(kind,titles[kind]):null;
  try{
   const result=await request();
   if(channel==='drop-files'&&result?.kind==='mods')id=begin('mods',titles.mods);
   if(id){
    if(result?.job)update(id,{job:result.job,state:result.queued?'queued':'running',instance:channel==='engine'?p.id:undefined,status:result.queued?'В очереди':'Подготовка файлов…'});
    else finish(id,'succeeded',kind==='scan'?(result.errors?'Найдены ошибки':result.warnings?'Есть предупреждения':'Проверка завершена'):result?.kind==='mods'?`Добавлено модов: ${result.count}`:'Готово');
   }
   if(channel==='engine'&&method==='state'){
    for(const item of snapshot){
     if(!item.job||item.ended)continue;
     const job=result.jobs?.find(j=>j.id===item.job);
     if(!job)continue;
     const terminal=!['running','queued'].includes(job.state);
     update(item.id,{state:job.state,status:job.error||job.status||'Подготовка…',instance:job.instance||item.instance,current:job.current,total:job.total,...(terminal?{ended:Date.now(),status:job.state==='succeeded'?'Готово':job.error||'Отменено'}:{})});
    }
   }
   return result;
  }catch(error){if(id)finish(id,'failed',String(error?.message||error));throw error}
 }
 return {observe,begin,finish,subscribe:fn=>{listeners.add(fn);return()=>listeners.delete(fn)},getSnapshot:()=>snapshot};
}
export const operations=createOperations();
export const gameReady=log=>/\[(?:Render thread|Client thread)\/INFO\].*(?:Sound engine started|OpenAL initialized|Created:.*(?:atlas|textures))/.test(log);
export function createMotionClock(){
 let elapsed=0,previous=null;
 return {get elapsed(){return elapsed},step(now){if(previous!==null)elapsed+=Math.max(0,Math.min(.05,(now-previous)/1000));previous=now;return elapsed},pause(){previous=null}};
}
