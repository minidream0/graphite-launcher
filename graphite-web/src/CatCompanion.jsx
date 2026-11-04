import React,{useEffect,useState,useSyncExternalStore} from 'react';
import {operations} from './operations';
import {CatArt} from './CatArt';
export function catPose(items,error,jobs,now=Date.now()){
 if(error||items.some(i=>i.state==='failed'&&i.ended&&now-i.ended<3200))return 'alert';
 const active=items.find(i=>!i.ended&&['pending','running'].includes(i.state));
 if(active&&['portal','route'].includes(active.kind))return 'stretch';
 if(active||jobs.some(j=>['running','queued'].includes(j.state)&&j.label!=='Launch Minecraft'))return 'watch';
 return 'sleep';
}
export default function CatCompanion({error,jobs=[]}){
 const items=useSyncExternalStore(operations.subscribe,operations.getSnapshot),[looking,setLooking]=useState(false),[paused,setPaused]=useState(document.hidden),[,refresh]=useState(0);
 const pose=looking?'look':catPose(items,error,jobs),lastFailure=items.filter(i=>i.state==='failed').at(-1);
 useEffect(()=>{const visible=()=>setPaused(document.hidden);document.addEventListener('visibilitychange',visible);return()=>document.removeEventListener('visibilitychange',visible)},[]);
 useEffect(()=>{if(!lastFailure?.ended)return;const timer=setTimeout(()=>refresh(n=>n+1),Math.max(0,3210-(Date.now()-lastFailure.ended)));return()=>clearTimeout(timer)},[lastFailure?.id,lastFailure?.ended]);
 useEffect(()=>{if(!looking)return;const timer=setTimeout(()=>setLooking(false),1600);return()=>clearTimeout(timer)},[looking]);
 function pet(e){const rect=e.currentTarget.getBoundingClientRect(),offset=e.detail?Math.max(-3,Math.min(3,(e.clientX-rect.left-rect.width/2)/25)):0;e.currentTarget.style.setProperty('--cat-look-x',`${offset}px`);e.currentTarget.style.setProperty('--cat-look-angle',`${offset}deg`);setLooking(true)}
 return <button type="button" className="cat-companion" aria-label="Погладить дымчатого кота" data-pose={pose} data-paused={paused||undefined} onClick={pet}><CatArt pose={pose}/><span className="sr-only">{pose==='sleep'?'Кот дремлет':pose==='alert'?'Кот насторожился':pose==='stretch'?'Кот потягивается':'Кот наблюдает'}</span></button>;
}
