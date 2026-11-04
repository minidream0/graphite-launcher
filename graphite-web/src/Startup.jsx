import React,{useCallback,useEffect,useRef,useState} from 'react';
import {version} from '../package.json';
import {Minus,X} from './icons';
import Workspace from './Workspace';
import {operations} from './operations';
import './startup.css';
import {useTheme} from './theme';
import SmokyStartup from './SmokyStartup';

function Startup({onReveal,onDone}){
 const root=useRef(null),canvas=useRef(null),flight=useRef(null),caption=useRef(null),finish=useRef(null);
 useEffect(()=>{
  const element=root.current,media=matchMedia('(prefers-reduced-motion: reduce)');
  let stopped=false,leaving=false,raf=0,timer=0,watchdog=0,scene=null,observer=null,flightAnimation=null,start=0;
  const close=()=>{if(stopped)return;stopped=true;cancelAnimationFrame(raf);clearTimeout(timer);clearTimeout(watchdog);observer?.disconnect();canvas.current?.removeEventListener('webglcontextlost',lost);scene?.dispose();scene=null;flightAnimation?.cancel();onDone()};
  const exit=(animate=true)=>{
   if(stopped||leaving)return;leaving=true;cancelAnimationFrame(raf);observer?.disconnect();
   element.dataset.phase='exit';onReveal();
   Object.assign(caption.current.style,{transition:'opacity 160ms ease-out, transform 160ms ease-out',opacity:'0',transform:'translate3d(0,-6px,0)'});
   timer=setTimeout(close,animate?680:0);
   if(animate&&scene){
    try{
    scene.frame(2.65);
    const {size}=scene.flightBounds(),rect=canvas.current.getBoundingClientRect(),target=document.querySelector('.brandmark img')?.getBoundingClientRect();
    if(target){
     const cx=rect.left+rect.width/2,cy=rect.top+rect.height/2;
     Object.assign(flight.current.style,{width:`${size}px`,height:`${size}px`,left:`${cx-size/2}px`,top:`${cy-size/2}px`});
     flightAnimation=flight.current.animate([{opacity:0,transform:'translate3d(0,0,0) scale(1)',offset:0},{opacity:1,transform:'translate3d(0,0,0) scale(1)',offset:.12},{opacity:1,transform:`translate3d(${target.left+target.width/2-cx}px,${target.top+target.height/2-cy}px,0) scale(${target.width/size})`,offset:1}],{duration:650,easing:'cubic-bezier(.65,0,.2,1)',fill:'forwards'});
    }
    }catch{close()}
   }
  };
  const lost=event=>{event.preventDefault();exit(false)};
  const skip=()=>exit(false),keyboard=e=>{if(e.key==='Escape'||e.key==='Enter'){e.preventDefault();skip()}},visibility=()=>{if(document.hidden)skip()},reduced=()=>{if(media.matches)skip()};
  finish.current=skip;
  document.addEventListener('keydown',keyboard);document.addEventListener('visibilitychange',visibility);media.addEventListener('change',reduced);
  canvas.current.addEventListener('webglcontextlost',lost);
  watchdog=setTimeout(()=>exit(false),5000);
  if(media.matches){element.dataset.mode='static';timer=setTimeout(()=>exit(false),180)}
  else import('./startup-scene').then(({createStartupScene})=>{
   if(stopped||leaving)return;
   try{
    scene=createStartupScene(canvas.current);element.dataset.mode='webgl';
    observer=new ResizeObserver(()=>{try{scene?.resize()}catch{exit(false)}});observer.observe(canvas.current);
    start=performance.now();
    const tick=now=>{
     if(stopped||leaving)return;
     const t=(now-start)/1000;try{scene.frame(t)}catch{exit(false);return}
     const phase=t<.65?'closeup':t<1.85?'assembly':'brand';if(element.dataset.phase!==phase)element.dataset.phase=phase;
     const reveal=Math.min(1,Math.max(0,(t-1.7)/.5));caption.current.style.opacity=String(reveal);caption.current.style.transform=`translate3d(0,${(1-reveal)*14}px,0)`;
     if(t>=2.65){exit(true);return}raf=requestAnimationFrame(tick);
    };
    raf=requestAnimationFrame(tick);
   }catch{element.dataset.mode='static';timer=setTimeout(()=>exit(false),220)}
  }).catch(()=>{if(!stopped&&!leaving){element.dataset.mode='static';timer=setTimeout(()=>exit(false),220)}});
  return()=>{
   stopped=true;finish.current=null;cancelAnimationFrame(raf);clearTimeout(timer);clearTimeout(watchdog);observer?.disconnect();
   document.removeEventListener('keydown',keyboard);document.removeEventListener('visibilitychange',visibility);media.removeEventListener('change',reduced);canvas.current?.removeEventListener('webglcontextlost',lost);scene?.dispose();flightAnimation?.cancel();
  };
 },[onReveal,onDone]);
 return <div ref={root} className="startup" role="region" aria-label="Запуск Graphite" data-phase="closeup" data-mode="loading">
  <div className="startup-backdrop"/>
  <div className="startup-windowbar" onMouseDown={e=>{if(e.button===0&&!e.target.closest('button')&&window.__TAURI__)window.launcher.window('drag')}}><span>Graphite</span><div><button aria-label="Свернуть" onClick={()=>window.launcher?.window('minimize')}><Minus size={15}/></button><button aria-label="Закрыть" onClick={()=>window.launcher?.window('close')}><X size={17}/></button></div></div>
  <canvas ref={canvas} className="startup-stage" aria-hidden="true"/>
  <img className="startup-static-logo" src="./graphite.svg" alt=""/>
  <img ref={flight} className="startup-flight" src="./graphite.svg" alt=""/>
  <div ref={caption} className="startup-caption"><h1>Graphite</h1><p>Minecraft Launcher</p></div>
  <footer className="startup-footer"><span>{version}</span><button onClick={()=>finish.current?.()}>Пропустить<kbd>Esc</kbd></button></footer>
 </div>
}
export default function App(){
 const theme=useTheme();
 const [booting,setBooting]=useState(true),[revealing,setRevealing]=useState(false);
 const reveal=useCallback(()=>setRevealing(true),[]),done=useCallback(()=>{setRevealing(true);setBooting(false)},[]);
 useEffect(()=>{if(booting)return;let alive=true;window.launcher?.tools('presentation.version',{}).then(previous=>{if(alive&&previous){const id=operations.begin('update','Новая версия Graphite');operations.finish(id,'succeeded',`${previous} → ${version}`)}}).catch(()=>{});return()=>{alive=false}},[booting]);
 return <><div className={`launcher-shell ${revealing?'launcher-revealed':''}`} inert={booting} aria-hidden={booting||undefined}><Workspace/></div>{booting&&(theme==='smoky'?<SmokyStartup onReveal={reveal} onDone={done}/>:<Startup onReveal={reveal} onDone={done}/>)}</>
}
