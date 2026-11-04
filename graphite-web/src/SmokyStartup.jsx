import React,{useEffect,useId,useRef,useState} from 'react';
import {version} from '../package.json';
import {CatArt} from './CatArt';
import {Minus,X} from './icons';
export default function SmokyStartup({onReveal,onDone}){
 const [phase,setPhase]=useState('arrive'),exit=useRef(null),id=useId();
 useEffect(()=>{
  let stopped=false,leaving=false,finishTimer;const timers=[],media=matchMedia('(prefers-reduced-motion: reduce)');
  const close=()=>{if(stopped)return;stopped=true;onDone()};
  const finish=(animate=true)=>{if(stopped||leaving)return;leaving=true;setPhase('exit');onReveal();finishTimer=setTimeout(close,animate&&!media.matches?480:0)};
  exit.current=()=>finish(false);const keyboard=e=>{if(e.key==='Escape'||e.key==='Enter'){e.preventDefault();finish(false)}},visible=()=>{if(document.hidden)finish(false)},reduced=()=>{if(media.matches)finish(false)};
  if(media.matches)timers.push(setTimeout(()=>finish(false),180));
  else{timers.push(setTimeout(()=>setPhase('push'),550),setTimeout(()=>setPhase('rest'),1800),setTimeout(()=>finish(),2700))}
  document.addEventListener('keydown',keyboard);document.addEventListener('visibilitychange',visible);media.addEventListener('change',reduced);
  return()=>{stopped=true;exit.current=null;timers.forEach(clearTimeout);clearTimeout(finishTimer);document.removeEventListener('keydown',keyboard);document.removeEventListener('visibilitychange',visible);media.removeEventListener('change',reduced)};
 },[onReveal,onDone]);
 return <div className="startup smoky-startup" role="region" aria-label="Запуск Graphite" data-phase={phase} data-mode="smoky">
  <div className="startup-backdrop"/>
  <div className="startup-windowbar" onMouseDown={e=>{if(e.button===0&&!e.target.closest('button')&&window.__TAURI__)window.launcher.window('drag')}}><span>Graphite</span><div><button aria-label="Свернуть" onClick={()=>window.launcher?.window('minimize')}><Minus size={15}/></button><button aria-label="Закрыть" onClick={()=>window.launcher?.window('close')}><X size={17}/></button></div></div>
  <div className="smoky-startup-scene" aria-hidden="true"><div className="smoky-startup-ledge"/><CatArt pose={phase==='arrive'?'sleep':'stretch'} className="smoky-startup-cat"/><svg className="smoky-startup-logo" viewBox="0 0 256 256"><defs><clipPath id={id}><path fillRule="evenodd" clipRule="evenodd" d="M0 0H256V256H0ZM146 23V117H237V23Z"/></clipPath><linearGradient id={`${id}-tile`} x1="147" y1="25" x2="235" y2="115" gradientUnits="userSpaceOnUse"><stop stopColor="#625f61"/><stop offset="1" stopColor="#4e4b4e"/></linearGradient></defs><image href="./graphite.svg" width="256" height="256" clipPath={`url(#${id})`}/><g className="smoky-logo-tile"><path d="M147 25h80l8 9v81h-88Z" fill={`url(#${id}-tile)`}/><path d="M147 25h80l8 9h-88Z" fill="#797576"/><path d="M227 34h8v81l-8-6Z" fill="#434144"/></g></svg></div>
  <div className="smoky-startup-word"><h1>Graphite</h1></div>
  <footer className="startup-footer"><span>{version}</span><button onClick={()=>exit.current?.()}>Пропустить<kbd>Esc</kbd></button></footer>
 </div>;
}
