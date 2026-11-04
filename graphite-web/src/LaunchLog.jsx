import React,{useEffect,useState} from 'react';
import {Copy,Check} from './icons';
import {message} from './errors';
import {diagnose} from './diagnose';
export default function LaunchLog({id,name,state}){
 const [log,setLog]=useState(''),[error,setError]=useState(''),[copied,setCopied]=useState(false);
 useEffect(()=>{let alive=true;async function refresh(){try{const data=await window.launcher.engine('instance.log',{id});if(alive)setLog(data.text)}catch(e){if(alive)setError(message(e))}}refresh();const timer=setInterval(refresh,1500);return()=>{alive=false;clearInterval(timer)}},[id]);
 const job=[...state.jobs].reverse().find(j=>j.label==='Launch Minecraft'&&j.instance===id);
 const cause=diagnose((job?.error||'')+'\n'+log);
 return <div className="installed-mods"><div className="installed-head"><span>{name} · текущий запуск</span><button className="secondary-button" disabled={!log} onClick={async()=>{try{await window.launcher.copyLog(id);setCopied(true);setTimeout(()=>setCopied(false),2000)}catch(e){setError(message(e))}}}>{copied?<Check size={16}/>:<Copy size={16}/>} {copied?'Скопировано':'Копировать'}</button></div>{error&&<p role="alert" className="panel-error">{error}</p>}{job?.error&&<p role="alert" className="panel-error">{job.error}</p>}{cause&&<p className="field-hint">{cause}</p>}<pre className="launch-log">{log||'Журнал появится после начала запуска.'}</pre></div>
}
