import React,{useEffect,useRef,useState} from 'react';
import {Globe,ArrowClockwise,PencilSimple,Play,Plus,Trash,SpinnerGap,DownloadSimple,GraphiteIcon,WarningCircle,Check} from './icons';
import {message} from './errors';
import {createPoller} from './poll';
import {matchingInstances,chooseInstance,normalizeServer,validateServer} from './server-form';
const api=window.launcher;
const checked=value=>value?new Date(value).toLocaleString('ru',{day:'numeric',month:'short',hour:'2-digit',minute:'2-digit'}):'';
function Field({name,label,error,children}){return <label className="form-field"><span>{label}</span>{React.cloneElement(children,{name,'aria-invalid':!!error,'aria-describedby':error?`server-${name}-error`:undefined})}{error&&<small className="server-field-error" id={`server-${name}-error`}>{error}</small>}</label>}

export default function Servers({instances,onStarted,onAccounts}){
 const [items,setItems]=useState(null),[form,setForm]=useState(null),[statuses,setStatuses]=useState({}),[busy,setBusy]=useState(''),[checking,setChecking]=useState(false),[error,setError]=useState(''),[errors,setErrors]=useState({}),[remove,setRemove]=useState('');
 const formElement=useRef(null),poller=useRef(null),currentItems=useRef(items);currentItems.current=items;
 useEffect(()=>{let alive=true;Promise.all([api.servers('list'),api.servers('cached-status')]).then(([list,cache])=>{if(alive){setStatuses(cache);setItems(list)}}).catch(e=>{if(alive)setError(message(e))});return()=>{alive=false}},[]);
 const signature=items?.map(i=>i.id+':'+i.address).join('|');
 useEffect(()=>{
  if(items===null)return;let alive=true;
  const poll=createPoller({read:async()=>{
   if(document.hidden)return null;setChecking(true);const results={};
   try{const list=currentItems.current||[];for(let offset=0;offset<list.length&&alive&&!document.hidden;offset+=4){await Promise.all(list.slice(offset,offset+4).map(async item=>{try{results[item.id]=await api.servers('status',{id:item.id})}catch(e){results[item.id]={online:false,error:message(e),checkedAt:Date.now()}}}));if(alive)setStatuses(previous=>({...previous,...results}))}return results}finally{if(alive)setChecking(false)}
  },accept:result=>{if(result)setStatuses(previous=>({...previous,...result}))},onError:e=>setError(message(e)),getDelay:()=>document.hidden?10000:30000});
  poller.current=poll;poll.refresh();const visible=()=>{if(!document.hidden)poll.refresh()};document.addEventListener('visibilitychange',visible);
  return()=>{alive=false;poll.stop();if(poller.current===poll)poller.current=null;document.removeEventListener('visibilitychange',visible)};
 },[signature]);
 const valid=matchingInstances(instances,''),choices=form?matchingInstances(instances,form.minecraft):[];
 function openForm(value){setError('');setErrors({});setForm({...value,instanceId:chooseInstance(instances,value.minecraft||'',value.instanceId||'')})}
 function change(name,value){setErrors(previous=>({...previous,[name]:null,...(name==='minecraft'?{instanceId:null}:{})}));setForm(previous=>({...previous,[name]:value,...(name==='minecraft'?{instanceId:chooseInstance(instances,value,previous.instanceId)}:{})}))}
 async function save(e){
  e.preventDefault();e.stopPropagation();const issues=validateServer(form,instances);setErrors(issues);setError('');
  if(Object.keys(issues).length){requestAnimationFrame(()=>formElement.current?.elements.namedItem(Object.keys(issues)[0])?.focus());return}
  setBusy('save');try{const value=normalizeServer(form),list=await api.servers('save',value);setItems(list);setStatuses(previous=>{const next={...previous};if(form.id&&items?.find(i=>i.id===form.id)?.address!==list.find(i=>i.id===form.id)?.address)delete next[form.id];return next});setForm(null)}catch(e){setError(message(e))}finally{setBusy('')}
 }
 async function bind(item,instanceId){setBusy(item.id);setError('');try{setItems(await api.servers('save',{...item,instanceId}))}catch(e){setError(message(e))}finally{setBusy('')}}
 async function action(method,id){setBusy(id);setError('');try{const result=await api.servers(method,{id});if(method==='remove'){setItems(result);setRemove('')}else onStarted()}catch(e){if(e.message.includes('AUTH_REQUIRED'))onAccounts();else setError(message(e))}finally{setBusy('')}}
 return <div className="servers-panel server-workspace">{error&&!form&&<p role="alert" className="panel-error">{error}</p>}
  <div className="servers-intro"><div><span className="eyebrow">Совместная игра</span><h3>Твои серверы</h3><p>Сервер, подходящая сборка и подключение — в одном месте.</p></div><GraphiteIcon id="graphite-v1-actions-16" size={64}/></div>
  <div className="installed-head"><span>{items?.length||0} сохранено</span><div className="manage-actions"><button type="button" className="secondary-button" disabled={checking||!items?.length} onClick={()=>poller.current?.refresh()}>{checking?<SpinnerGap className="spin" size={16}/>:<ArrowClockwise size={20}/>}Обновить статус</button><button type="button" className="primary-button" disabled={!!busy} onClick={()=>openForm({name:'',address:'',instanceId:'',minecraft:'',project:''})}><Plus size={16}/>Добавить сервер</button></div></div>
  {form&&<form ref={formElement} className="server-form" onSubmit={save} noValidate>
   <fieldset disabled={busy==='save'}>
   <div className="server-form-grid">
    <Field name="name" label="Название · обязательно" error={errors.name}><input aria-label="Название сервера" required maxLength="80" value={form.name} onChange={e=>change('name',e.target.value)}/></Field>
    <Field name="address" label="Адрес · обязательно" error={errors.address}><input aria-label="Адрес сервера" required maxLength="255" placeholder="play.example.org:25565" value={form.address} onChange={e=>change('address',e.target.value)}/></Field>
    <Field name="minecraft" label="Версия Minecraft · необязательно" error={errors.minecraft}><input aria-label="Версия Minecraft сервера" maxLength="80" placeholder="Например, 1.20.1" value={form.minecraft||''} onChange={e=>change('minecraft',e.target.value)}/></Field>
    <Field name="project" label="Сборка Modrinth · необязательно" error={errors.project}><input aria-label="Сборка Modrinth сервера" maxLength="100" placeholder="ID или slug проекта" value={form.project||''} onChange={e=>change('project',e.target.value)}/></Field>
   </div>
   <p className="field-hint">Для добавления достаточно названия и адреса. Версию и сборку Modrinth можно указать позже. Совпадение версии Minecraft не гарантирует совпадение модов сервера.</p>
   <Field name="instanceId" label="Сборка для подключения" error={errors.instanceId}><select aria-label="Сборка сервера" value={form.instanceId} onChange={e=>change('instanceId',e.target.value)}><option value="">{form.minecraft&&choices.length>1?`Выбери одну из сборок ${form.minecraft}`:'Без привязки — выбрать позже'}</option>{choices.map(i=><option key={i.id} value={i.id}>{i.name} · {i.version} · {i.loader}</option>)}</select></Field>
   {form.minecraft&&!choices.length&&<p className="field-hint">Установленной сборки {form.minecraft.trim()} пока нет. Сервер можно сохранить без привязки.</p>}
   </fieldset>
   {Object.values(errors).some(Boolean)&&<p className="panel-error" role="alert">Проверь отмеченные поля. Сервер ещё не сохранён.</p>}{error&&<p className="panel-error" role="alert">{error}</p>}
   <div className="manage-actions"><button type="submit" className="primary-button" disabled={!!busy}>{busy==='save'?'Сохраняем…':'Сохранить'}</button><button type="button" className="secondary-button" disabled={!!busy} onClick={()=>{setForm(null);setErrors({});setError('')}}>Отмена</button></div>
  </form>}
  <div className="server-list">{items?.map(item=>{
   const status=statuses[item.id],instance=valid.find(i=>i.id===item.instanceId),matches=matchingInstances(instances,item.minecraft),mismatch=!!(instance&&item.minecraft&&instance.version.trim()!==item.minecraft.trim());
   return <article className="server-card" key={item.id}>
    <div className="server-card-top"><div className="server-emblem"><Globe size={43}/></div><div className="server-identity"><h4>{item.name}</h4><p>{item.address}</p></div><span className={`server-presence ${status?.online?'online':''}`}><i/>{status?status.online?'Онлайн':'Недоступен':checking?'Проверяем…':'Не проверен'}</span><div className="server-card-tools"><button type="button" className="glyph" aria-label={`Изменить ${item.name}`} disabled={!!busy} onClick={()=>openForm({...item,minecraft:item.minecraft||'',project:item.project||''})}><PencilSimple size={22}/></button><button type="button" className="glyph" aria-label={`Удалить ${item.name}`} disabled={!!busy} onClick={()=>setRemove(item.id)}><Trash size={22}/></button></div></div>
    {status?.checkedAt&&<p className="server-status-time">Последняя проверка: {checked(status.checkedAt)}{checking?' · обновляем…':''}</p>}
    {status?.online&&<><div className="server-metrics"><div><span>Игроки</span><strong>{status.players??'—'}<small> / {status.max??'—'}</small></strong></div><div><span>Отклик</span><strong>{status.latency}<small> мс</small></strong></div><div><span>Версия сервера</span><strong>{status.version}</strong></div></div><p className="server-motd">{status.motd}</p></>}
    {status&&!status.online&&<p className="field-hint">{status.error}</p>}{status?.cacheWarning&&<p className="field-hint">{status.cacheWarning}</p>}
    <div className={`server-binding ${mismatch?'mismatch':''}`}><GraphiteIcon id={`graphite-v1-actions-${mismatch?'18':'13'}`} size={28}/><span><strong>{instance?.name||'Сборка не выбрана'}</strong><small>{instance?`Minecraft ${instance.version} · ${instance.loader}`:item.minecraft&&matches.length?`Версия ${item.minecraft} есть в библиотеке — выбери сборку ниже`:item.minecraft?`Для сервера нужна ${item.minecraft}`:'Выбери установленную сборку ниже'}</small></span>{mismatch?<span className="server-compatibility"><WarningCircle size={15}/>{matches.length?`Выбери сборку ${item.minecraft}`:`Нужна ${item.minecraft}`}</span>:instance&&item.minecraft?<span className="server-compatibility"><Check size={15}/>Версия совпадает</span>:null}</div>
    <label className="form-field server-binding-picker"><span>Играть через сборку</span><select aria-label={`Сборка для ${item.name}`} value={instance?.id||''} disabled={!!busy} onChange={e=>bind(item,e.target.value)}><option value="">Без привязки</option>{mismatch&&<option value={instance.id} disabled>{instance.name} · {instance.version} — другая версия</option>}{matches.map(i=><option key={i.id} value={i.id}>{i.name} · {i.version} · {i.loader}</option>)}</select></label>
    <div className="server-card-footer">{item.project&&<button type="button" className="secondary-button" disabled={!!busy} onClick={()=>action('prepare',item.id)}><DownloadSimple size={22}/>{instance?'Новая копия сборки':'Подготовить сборку'}</button>}<button type="button" className="primary-button" disabled={!!busy||!instance||mismatch} onClick={()=>action('launch',item.id)}><Play size={15}/>Подключиться</button></div>
    {remove===item.id&&<div className="inline-confirm"><span>Убрать из избранного?</span><button type="button" className="text-button" onClick={()=>setRemove('')}>Отмена</button><button type="button" className="text-button danger" disabled={!!busy} onClick={()=>action('remove',item.id)}>Удалить</button></div>}
   </article>
  })}{items?.length===0&&!form&&<div className="tool-empty"><Globe size={52}/><h3>Добавь первый сервер</h3><p>Укажи адрес и привяжи сборку для подключения.</p></div>}{items===null&&!error&&<div className="panel-loading"><SpinnerGap className="spin"/>Загрузка…</div>}</div>
 </div>;
}
