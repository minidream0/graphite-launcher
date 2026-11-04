import React,{useCallback,useEffect,useLayoutEffect,useRef,useState,useSyncExternalStore} from 'react';
import {createPortal} from 'react-dom';
import {operations,gameReady,createMotionClock} from './operations';
import {X} from './icons';
import './operation-stage.css';

export default function OperationStage(){
 const items=useSyncExternalStore(operations.subscribe,operations.getSnapshot);
 const [hidden,setHidden]=useState([]),[displayed,setDisplayed]=useState(null),[closing,setClosing]=useState(null),[revision,refresh]=useState(0),[mode,setMode]=useState('static');
 const [container]=useState(()=>document.createElement('div'));
 const host=useRef(null),canvas=useRef(null),model=useRef(null);
 const live=items.find(i=>i.id===displayed?.id),item=live||displayed;
 const dismiss=useCallback((id,consume=true)=>setClosing(previous=>previous||{id,consume}),[]);
 useLayoutEffect(()=>{
  const locate=()=>{
   const dialogs=document.querySelectorAll('dialog:modal'),parent=dialogs[dialogs.length-1]||document.body;
   // Preserve the canvas and its GPU context when moving between modal panels.
   if(container.parentNode!==parent){parent.appendChild(container);try{host.current?.showPopover()}catch{}}
  };
  locate();const observer=new MutationObserver(locate);observer.observe(document.body,{subtree:true,childList:true,attributes:true,attributeFilter:['open']});
  return()=>{observer.disconnect();container.remove()};
 },[container]);
 useEffect(()=>{
  if(displayed)return;
  const now=Date.now(),candidate=items.find(i=>!hidden.includes(i.id)&&i.state!=='queued'&&(!i.ended||now-i.ended<2400)&&(i.state!=='pending'||now-i.started>=180));
  if(candidate){setDisplayed(candidate);return}
  if(items.some(i=>!hidden.includes(i.id)&&i.state==='pending')){const timer=setTimeout(()=>refresh(n=>n+1),185);return()=>clearTimeout(timer)}
 },[items,hidden,displayed,revision]);
 useEffect(()=>{if(!item)return;if(!live||item.state==='queued'){dismiss(item.id,false);return}if(item.ended){const timer=setTimeout(()=>dismiss(item.id),Math.max(0,2400-(Date.now()-item.ended)));return()=>clearTimeout(timer)}},[item?.id,item?.state,item?.ended,!!live,dismiss]);
 useEffect(()=>{
  if(!closing||!item)return;
  let alive=true,finished=false;
  const finish=()=>{if(!alive||finished)return;finished=true;if(closing.consume)setHidden(ids=>[...ids.slice(-63),closing.id]);setDisplayed(null);setClosing(null)};
  if(matchMedia('(prefers-reduced-motion: reduce)').matches){finish();return}
  let animation;try{const element=host.current,style=getComputedStyle(element),from={opacity:style.opacity,transform:style.transform};for(const active of element.getAnimations())active.cancel();animation=element.animate([from,{opacity:0,transform:'translateY(8px) scale(.985)'}],{duration:260,easing:'cubic-bezier(.4,0,.2,1)',fill:'forwards'});animation.finished.then(finish,finish)}catch{finish()}
  const timer=setTimeout(finish,320);return()=>{alive=false;clearTimeout(timer);animation?.cancel()};
 },[closing,item?.id]);
 useEffect(()=>{
  if(!item)return;
  try{host.current.showPopover();if(!matchMedia('(prefers-reduced-motion: reduce)').matches)host.current.animate([{opacity:0,transform:'translateY(12px) scale(.975)'},{opacity:1,transform:'translateY(0) scale(1)'}],{duration:400,easing:'cubic-bezier(.2,.8,.2,1)'})}catch{}
 },[item?.id]);
 useEffect(()=>{
  if(!item)return;setMode('static');
  let disposed=false,scene,frame=0,observer,generation=0;const clock=createMotionClock(),media=matchMedia('(prefers-reduced-motion: reduce)');
  const stop=()=>{generation++;cancelAnimationFrame(frame);observer?.disconnect();scene?.dispose();scene=null;clock.pause();setMode('static')};
  const tick=t=>{if(disposed||!scene)return;try{scene.frame(clock.step(t),model.current);frame=requestAnimationFrame(tick)}catch{stop()}};
  const visibility=()=>{cancelAnimationFrame(frame);clock.pause();if(!document.hidden&&scene)frame=requestAnimationFrame(tick)};
  const lost=event=>{event.preventDefault();stop()};
  async function start(){
   if(media.matches||disposed)return;const attempt=++generation;
   try{const {createOperationScene}=await import('./operation-scene');if(disposed||media.matches||attempt!==generation)return;scene=createOperationScene(canvas.current,item.kind);scene.frame(clock.elapsed,model.current);observer=new ResizeObserver(()=>{try{scene?.resize()}catch{stop()}});observer.observe(canvas.current);setMode('webgl');visibility()}catch{stop()}
  }
  const preference=()=>{stop();if(!media.matches)start()};
  const element=canvas.current;element.addEventListener('webglcontextlost',lost);media.addEventListener('change',preference);document.addEventListener('visibilitychange',visibility);start();
  return()=>{disposed=true;observer?.disconnect();element.removeEventListener('webglcontextlost',lost);media.removeEventListener('change',preference);document.removeEventListener('visibilitychange',visibility);cancelAnimationFrame(frame);scene?.dispose()};
 },[item?.id]);
 model.current=item;
 useEffect(()=>{
  if(!item||!['portal','route'].includes(item.kind)||item.ended||!item.instance)return;
  let alive=true,timer;
  async function read(){try{if(!document.hidden){const value=await window.launcher.engine('instance.log',{id:item.instance});if(alive&&gameReady(value.text||'')){operations.finish(item.id,'succeeded','Minecraft запущен');return}}}catch{}if(alive)timer=setTimeout(read,1600)}
  read();return()=>{alive=false;clearTimeout(timer)};
 },[item?.id,item?.instance,item?.ended]);
 if(!item)return null;
 const terminal=!!item.ended,progress=item.total>0?Math.max(0,Math.min(1,item.current/item.total)):null;
 return createPortal(<aside key={item.id} ref={host} popover="manual" className={`operation-stage operation-${item.kind} ${terminal?'operation-ended':''}`} data-mode={mode} data-kind={item.kind} aria-label={item.title}>
  <div className="operation-art" aria-hidden="true"/>
  <div className="operation-viewport" aria-hidden="true"><canvas ref={canvas}/><img className="operation-static" src="./graphite.svg" alt=""/></div>
  <button className="glyph operation-hide" aria-label="Скрыть анимацию" onClick={()=>dismiss(item.id)}><X size={17}/></button>
  <div className="operation-caption" role="status" aria-live="polite"><span className="operation-eyebrow">Graphite <i/> {terminal?item.state==='failed'?'Ошибка':item.state==='cancelled'?'Отменено':'Завершено':'Выполняется'}</span><h3>{item.title}</h3><p>{item.status}</p></div>
  {!terminal&&progress!==null&&<div className="operation-progress" role="progressbar" aria-label="Прогресс этапа" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(progress*100)}><span style={{transform:`scaleX(${progress})`}}/></div>}
 </aside>,container);
}
