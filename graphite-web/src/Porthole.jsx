import React,{useEffect,useRef,useState} from 'react';
import {Globe,Play,Copy,ArrowClockwise,FolderOpen,SpinnerGap,Check} from './icons';
import {createPoller,retainSnapshot} from './poll';
import {message} from './errors';
import './porthole.css';

const api=window.launcher;
const phases={idle:'Соединение не открыто',starting:'Создаём комнату',hosting:'Комната открыта',connecting:'Подключаемся',connected:'Соединение установлено',error:'Не удалось подключиться'};

export default function Porthole({instances,onStarted,onAccounts}){
 const [state,setState]=useState(null),[mode,setMode]=useState('host'),[instance,setInstance]=useState(instances[0]?.id||''),[port,setPort]=useState(''),[target,setTarget]=useState(''),[password,setPassword]=useState(''),[error,setError]=useState(''),[busy,setBusy]=useState(''),[copied,setCopied]=useState('');
 const current=useRef(null),poll=useRef(null),alive=useRef(false),copyTimer=useRef(null);
 useEffect(()=>{
  alive.current=true;
  const read=createPoller({read:()=>api.porthole('status'),accept:value=>{current.current=value;setState(previous=>retainSnapshot(previous,value))},onError:e=>setError(message(e)),getDelay:()=>current.current?.active?900:5000});
  poll.current=read;read.refresh();
  return()=>{alive.current=false;read.stop();clearTimeout(copyTimer.current)};
 },[]);
 async function action(method,p={}){
  setBusy(method);setError('');
  try{
   const result=await api.porthole(method,p);
   if(!alive.current)return;
   if(['host','join','stop','pick','refresh'].includes(method)){setState(result);current.current=result}
   if(method==='detect')setPort(String(result.port));
   if(method==='launch')onStarted();
   if(method==='copy'){setCopied(p.kind);clearTimeout(copyTimer.current);copyTimer.current=setTimeout(()=>setCopied(''),1800)}
   poll.current?.refresh();return result;
  }catch(e){
   if(alive.current){if(e.message.includes('AUTH_REQUIRED'))onAccounts();else setError(message(e))}
  }finally{if(alive.current)setBusy('')}
 }
 async function submit(e){
  e.preventDefault();
  if(!/^\d+$/.test(port)||Number(port)<1||Number(port)>65535){setError('Укажи порт мира от 1 до 65535');return}
  if(mode==='join'&&!target.trim()){setError('Вставь код комнаты друга');return}
  await action(mode,{port:Number(port),target:target.trim(),password,id:instance});
  if(alive.current)setPassword('');
 }
 async function launchWorld(){
  setBusy('game');setError('');
  try{await api.engine('launch',{id:instance});if(alive.current)onStarted()}
  catch(e){if(alive.current){if(e.message.includes('AUTH_REQUIRED'))onAccounts();else setError(message(e))}}
  finally{if(alive.current)setBusy('')}
 }
 const active=state?.active,locked=!!busy,hosting=state?.mode==='host',ready=state?.phase==='connected';
 return <div className="porthole">
  <header className="porthole-heading"><div><h3>Играть с друзьями</h3><p>Открой свой мир или присоединись к другу через Porthole.</p></div><Globe size={48}/></header>
  {(error||state?.error)&&<p className="panel-error" role="alert">{error||state.error}</p>}
  {!state?<div className="panel-loading"><SpinnerGap className="spin"/>Проверяем Porthole…</div>:!state.installed?
   <section className="porthole-install"><h4>Установи Porthole в Steam</h4><p>Это бесплатное приложение соединит ваши компьютеры. Вход выполняется через запущенный Steam.</p><div className="manage-actions"><button className="primary-button" disabled={locked} onClick={()=>action('install')}>Установить Porthole</button><button className="secondary-button" disabled={locked} onClick={()=>action('refresh')}><ArrowClockwise size={20}/>Проверить установку</button><button className="text-button" disabled={locked} onClick={()=>action('pick')}><FolderOpen size={20}/>Выбрать porthole.exe</button></div></section>:
   <>
    <div className="porthole-provider"><span className={state.steamId?'porthole-steam online':'porthole-steam'}><i/>{state.steamId?'Steam подключён':'Porthole найден'}</span><button className="text-button" disabled={locked} onClick={()=>action('steam')}>Открыть Steam</button><button className="text-button" disabled={locked} onClick={()=>action('open')}>Открыть Porthole</button></div>
    {!active?
     <form className="porthole-form" onSubmit={submit} noValidate>
      <div className="porthole-modes" role="group" aria-label="Способ подключения"><button type="button" aria-pressed={mode==='host'} onClick={()=>{setMode('host');setPort('');setError('')}}>Мой мир</button><button type="button" aria-pressed={mode==='join'} onClick={()=>{setMode('join');setPort('25565');setError('')}}>К другу</button></div>
      <label className="form-field"><span>Сборка Minecraft</span><select aria-label="Сборка для совместной игры" value={instance} disabled={locked} onChange={e=>setInstance(e.target.value)}><option value="">Выбрать сборку</option>{instances.filter(i=>!i.broken).map(i=><option key={i.id} value={i.id}>{i.name} · {i.version} · {i.loader}</option>)}</select></label>
      {mode==='host'?<div className="porthole-instruction"><span className="porthole-step">1</span><div><strong>Открой мир для сети</strong><p>В Minecraft: Esc → «Открыть для сети». Игра покажет порт.</p></div><button type="button" className="secondary-button" disabled={locked||!instance} onClick={launchWorld}><Play size={16}/>Запустить Minecraft</button></div>:
       <label className="form-field"><span>Код комнаты</span><input aria-label="Код комнаты Porthole" autoComplete="off" spellCheck={false} maxLength={128} placeholder="Код от друга" value={target} disabled={locked} onChange={e=>setTarget(e.target.value)}/></label>}
      <div className="porthole-fields">
       <label className="form-field"><span>{mode==='host'?'Порт из Minecraft':'Порт мира друга'}</span><div className="porthole-port"><input aria-label="Порт мира Porthole" type="text" inputMode="numeric" maxLength={5} placeholder={mode==='host'?'Например, 51234':'25565'} value={port} disabled={locked} onChange={e=>setPort(e.target.value)}/>{mode==='host'&&<button type="button" className="secondary-button" disabled={locked||!instance} onClick={()=>action('detect',{id:instance})}>{busy==='detect'?<SpinnerGap className="spin"/>:<ArrowClockwise size={20}/>}Найти</button>}</div></label>
       <label className="form-field"><span>Пароль комнаты · если нужен</span><input aria-label="Пароль комнаты Porthole" type="password" autoComplete="new-password" maxLength={128} value={password} disabled={locked} onChange={e=>setPassword(e.target.value)}/></label>
      </div>
      {mode==='join'&&<p className="field-hint">Порт указан в комнате хозяина. В обеих сборках должны совпадать версия игры и необходимые моды.</p>}
      <div className="porthole-submit"><button type="submit" className="primary-button" disabled={locked}>{locked&&<SpinnerGap className="spin" size={16}/>} {mode==='host'?'Открыть комнату':'Подключиться к комнате'}</button></div>
     </form>:
     <section className="porthole-session" aria-live="polite">
      <div className="porthole-session-head"><span className={'porthole-connection '+(ready||state.phase==='hosting'?'online':'')}><i/>{phases[state.phase]||'Соединение'}</span><button className="secondary-button" disabled={locked} onClick={()=>action('stop')}>{hosting?'Закрыть комнату':'Отключиться'}</button></div>
      {!hosting&&state.offeredPort&&state.offeredPort!==state.port&&!state.address&&<p className="panel-error" role="alert">Мир друга открыт на порту {state.offeredPort}, а указан {state.port}. Отключись и введи порт {state.offeredPort}.</p>}
      {hosting&&state.code&&<><span className="porthole-caption">Код для друга</span><div className="porthole-code"><strong>{state.code}</strong><button className="secondary-button" aria-label="Скопировать код комнаты" disabled={locked} onClick={()=>action('copy',{kind:'code'})}>{copied==='code'?<Check size={18}/>:<Copy size={22}/>}Копировать</button></div><p className="field-hint">Друг выбирает «К другу» и вводит этот код и порт <strong>{state.port}</strong>.</p></>}
      {!hosting&&state.address&&<><span className="porthole-caption">Адрес для Minecraft</span><div className="porthole-code"><strong>{state.address}</strong><button className="secondary-button" disabled={locked} onClick={()=>action('copy',{kind:'address'})}>{copied==='address'?<Check size={18}/>:<Copy size={22}/>}Копировать</button></div><label className="form-field"><span>Сборка для подключения</span><select aria-label="Сборка для совместной игры" value={instance} disabled={locked} onChange={e=>setInstance(e.target.value)}><option value="">Выбрать сборку</option>{instances.filter(i=>!i.broken).map(i=><option key={i.id} value={i.id}>{i.name} · {i.version}</option>)}</select></label><button className="primary-button" disabled={locked||!instance||!ready} onClick={()=>action('launch',{id:instance})}><Play size={16}/>Играть</button></>}
      {(!hosting&&!state.address||hosting&&!state.code)&&state.phase!=='error'&&<div className="porthole-wait"><SpinnerGap className="spin"/><p>Ожидаем ответ Porthole. Steam должен быть запущен.</p></div>}
      <div className="porthole-session-meta">{state.steamId&&<span>Steam ID <code>{state.steamId}</code></span>}{state.members?.length>0&&<span>В комнате: {state.members.length}</span>}{state.ping>=0&&state.ping!=null&&<span>{state.ping} мс · {state.relayed?'Через Steam Relay':'Напрямую'}</span>}</div>
      <details className="porthole-details"><summary>Журнал соединения</summary><pre>{state.events?.map(e=>JSON.stringify(e)).join('\n')||'Событий пока нет'}</pre></details>
     </section>}
   </>}
 </div>;
}
