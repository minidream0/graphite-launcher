import React,{useEffect,useRef,useState} from 'react';
import {ImportIcon,SpinnerGap,WarningCircle} from './icons';
export default function FileDrop({selected,name,onResult,resourceKind,resourceId}){
 const [active,setActive]=useState(false),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const layer=useRef(null),current=useRef({selected,onResult,resourceKind,resourceId}),depth=useRef(0),working=useRef(false);current.current={selected,onResult,resourceKind,resourceId};
 useEffect(()=>{const files=e=>Array.from(e.dataTransfer?.types||[]).includes('Files');
  const enter=e=>{if(files(e)){e.preventDefault();depth.current++;setActive(true);setError('')}};
  const over=e=>{if(files(e)){e.preventDefault();e.dataTransfer.dropEffect='copy'}};
  const leave=e=>{if(files(e)&&--depth.current<=0){depth.current=0;setActive(false)}};
  const reset=()=>{depth.current=0;setActive(false)};
  const drop=async e=>{if(!files(e))return;e.preventDefault();e.stopPropagation();reset();if(working.current)return;working.current=true;setBusy(true);setError('');try{current.current.onResult(await window.launcher.dropFiles(Array.from(e.dataTransfer.files),current.current.resourceId||current.current.selected,current.current.resourceKind))}catch(e){setError(e.message.replace(/^Error invoking remote method '[^']+': (?:Error: )?/,''))}finally{working.current=false;setBusy(false)}};
  const escape=e=>{if(e.key==='Escape'){reset();setError('')}};
  let disposed=false,unlisten,unerror;
  if(window.launcher?.onFileDrop){
   window.launcher.onFileDrop(async event=>{if(event.type==='enter'){setActive(true);setError('')}if(event.type==='leave')reset();if(event.type!=='drop')return;reset();if(working.current)return;working.current=true;setBusy(true);setError('');try{current.current.onResult(await window.launcher.dropFiles(event.paths,current.current.resourceId||current.current.selected,current.current.resourceKind))}catch(e){setError(e.message)}finally{working.current=false;setBusy(false)}}).then(fn=>{if(disposed)fn();else unlisten=fn});
   window.launcher.onError(value=>setError(String(value))).then(fn=>{if(disposed)fn();else unerror=fn});
   document.addEventListener('keydown',escape);window.addEventListener('blur',reset);
   return()=>{disposed=true;unlisten?.();unerror?.();document.removeEventListener('keydown',escape);window.removeEventListener('blur',reset)};
  }
  document.addEventListener('dragenter',enter);document.addEventListener('dragover',over);document.addEventListener('dragleave',leave);document.addEventListener('drop',drop,true);document.addEventListener('keydown',escape);window.addEventListener('blur',reset);
  return()=>{document.removeEventListener('dragenter',enter);document.removeEventListener('dragover',over);document.removeEventListener('dragleave',leave);document.removeEventListener('drop',drop,true);document.removeEventListener('keydown',escape);window.removeEventListener('blur',reset)};
 },[]);
 useEffect(()=>{if(active||busy||error)layer.current.showPopover();else layer.current.hidePopover()},[active,busy,error]);
 return <div ref={layer} popover="manual" className={`file-drop ${error?'file-drop-error':''}`} role={error?'alert':active||busy?'status':undefined}><div>{error?<WarningCircle size={36}/>:busy?<SpinnerGap className="spin" size={36}/>:<ImportIcon size={40}/>}<strong>{error?'Не удалось добавить файлы':busy?'Добавляем файлы…':'Отпусти файлы здесь'}</strong><p>{error||(resourceKind?`ZIP — ${resourceKind==='shader'?'шейдеры':'ресурс-паки'}`:'.mrpack / ZIP — импорт сборки')}</p>{!error&&!resourceKind&&<p>{name?`.jar — в сборку «${name}»`:'.jar — сначала выбери установленную сборку'}</p>}{error&&<button className="secondary-button" onClick={()=>setError('')}>Закрыть</button>}</div></div>;
}
