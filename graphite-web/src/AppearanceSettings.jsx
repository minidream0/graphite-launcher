import React,{useState} from 'react';
import {useTheme,saveTheme} from './theme';
import {CatArt} from './CatArt';
import {message} from './errors';
export default function AppearanceSettings(){
 const theme=useTheme(),[busy,setBusy]=useState(false),[error,setError]=useState('');
 async function choose(value){if(value===theme||busy)return;setBusy(true);setError('');try{await saveTheme(value)}catch(e){setError(message(e))}finally{setBusy(false)}}
 function keyboard(e){if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key)||busy)return;e.preventDefault();const buttons=Array.from(e.currentTarget.querySelectorAll('[role=radio]')),index=buttons.indexOf(document.activeElement),next=buttons[(index+1)%buttons.length];next?.focus();next?.click()}
 return <section className="form-section appearance-settings"><div className="section-heading"><span>Внешний вид</span><span>Сохраняется сразу</span></div><div className="theme-options" role="radiogroup" aria-label="Тема лаунчера" onKeyDown={keyboard}><button type="button" role="radio" aria-checked={theme==='graphite'} aria-label="Тема Graphite" tabIndex={theme==='graphite'?0:-1} aria-disabled={busy} onClick={()=>choose('graphite')}><span className="theme-preview theme-preview-graphite"><img src="./graphite.svg" alt=""/></span><strong>Graphite</strong><small>Графит и коралл</small></button><button type="button" role="radio" aria-checked={theme==='smoky'} aria-label="Тема Дымчатый" tabIndex={theme==='smoky'?0:-1} aria-disabled={busy} onClick={()=>choose('smoky')}><span className="theme-preview theme-preview-smoky"><CatArt/></span><strong>Дымчатый</strong><small>Тёплый вечер с котом</small></button></div>{error&&<p className="panel-error" role="alert">{error}</p>}</section>;
}
