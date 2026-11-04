import {useSyncExternalStore} from 'react';
let current='graphite';const listeners=new Set();
const subscribe=fn=>{listeners.add(fn);return()=>listeners.delete(fn)};
export const themes=[{id:'graphite',name:'Graphite'},{id:'smoky',name:'Дымчатый'}];
export function applyTheme(value){
 current=value==='smoky'?'smoky':'graphite';document.documentElement.dataset.theme=current;
 for(const listener of listeners)listener();return current;
}
export const getTheme=()=>current;
export function useTheme(){return useSyncExternalStore(subscribe,getTheme)}
export async function initializeTheme(api=window.launcher){try{const value=await api?.tools?.('appearance.get',{});return applyTheme(value?.theme)}catch{return applyTheme('graphite')}}
export async function saveTheme(value,api=window.launcher){const result=await api.tools('appearance.set',{theme:value});return applyTheme(result.theme)}
