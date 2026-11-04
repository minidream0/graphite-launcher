import React,{useId} from 'react';
import catalog from './smoky-icon-catalog.json';
export const smokyIcons=catalog.sheets.flatMap(sheet=>sheet.frames.map((frame,index)=>({key:`graphite-v1-${sheet.group}-${String(index+1+sheet.offset).padStart(2,'0')}`,sheet,frame})));
export const smokyByKey=new Map(smokyIcons.map(icon=>[icon.key,icon]));
export default function SmokyIcon({id,size=26,className='',...props}){
 const icon=smokyByKey.get(id),clip=useId();if(!icon)return null;
 const {sheet,frame:[x,y,width,height]}=icon;
 return <svg {...props} width={size} height={size} viewBox={`${x} ${y} ${width} ${height}`} className={`graphite-icon smoky-icon ${className}`} data-smoky-icon={id} aria-hidden={props['aria-label']?undefined:true} focusable="false"><defs><clipPath id={clip}><rect x={x} y={y} width={width} height={height}/></clipPath><filter id={`${clip}-alpha`}><feComponentTransfer><feFuncA type="linear" slope="2" intercept="-1"/></feComponentTransfer></filter></defs><image href={`./smoky-icons/${sheet.file}`} width={sheet.width} height={sheet.height} clipPath={`url(#${clip})`} filter={`url(#${clip}-alpha)`}/></svg>;
}
