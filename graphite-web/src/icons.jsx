import React,{useId,useState} from 'react';
import catalog from './icon-catalog.json';
import {useTheme} from './theme';
import SmokyIcon,{smokyByKey} from './SmokyIcon';
import {Play as BasePlay,Check as BaseCheck,WarningCircle as BaseWarning,ClockCounterClockwise as BaseHistory} from '@phosphor-icons/react';
export {ArrowDown,ArrowUpRight,ArrowRight,ArrowLeft,Minus,X,CaretRight,Plus,SpinnerGap,DotsThree,Keyboard,Command,MagnifyingGlass} from '@phosphor-icons/react';

export const graphiteIcons=catalog.sheets.flatMap(sheet=>sheet.frames.map((frame,index)=>({key:`graphite-v1-${sheet.id}-${String(index+1).padStart(2,'0')}`,name:sheet.labels[index],sheet,frame})));
const byKey=new Map(graphiteIcons.map(icon=>[icon.key,icon]));
const key=(sheet,index)=>`graphite-v1-${sheet}-${String(index).padStart(2,'0')}`;
export const builtinKeys=new Set('bee brick chicken creeper diamond dirt enderman enderpearl fabricmc flame forge fox ftb_glow ftb_logo gear gold grass herobrine infinity iron liteloader magitech meat modrinth neoforged netherstar planks prismlauncher quiltmc skeleton squarecreeper steve stone tnt default minecraft'.split(' ').flatMap(name=>[name,name+'_legacy']));

export function GraphiteIcon({id,size=26,className='',weight,...props}){
 const icon=byKey.get(id)||graphiteIcons[0],{sheet,frame:[x,y,width,height]}=icon;
 const clip=useId();
 const theme=useTheme();
 if(theme==='smoky'&&smokyByKey.has(icon.key))return <SmokyIcon {...props} size={size} className={className} id={icon.key}/>;
 return <svg {...props} aria-hidden={props['aria-label']?undefined:true} width={size} height={size} viewBox={`${x} ${y} ${width} ${height}`} className={`graphite-icon ${className}`} data-graphite-icon={icon.key} focusable="false"><defs><clipPath id={clip}><rect x={x} y={y} width={width} height={height}/></clipPath><filter id={`${clip}-matte`} colorInterpolationFilters="sRGB"><feColorMatrix type="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  3.4 11.4 1.2 0 -2.4"/></filter></defs><image clipPath={`url(#${clip})`} filter={`url(#${clip}-matte)`} href={`./graphite-icons/${sheet.file}`} width={sheet.width} height={sheet.height}/></svg>;
}
export function InstanceIcon({icon,icons={},loader}){
 if(byKey.has(icon))return <GraphiteIcon id={icon} size={38}/>;
 if(icons[icon]&&!builtinKeys.has(icon))return <img src={icons[icon]} alt="" width="38" height="38"/>;
 const category={fabricmc:2,forge:3,neoforged:4,quiltmc:5,modrinth:6,ftb_logo:6,ftb_glow:6,magitech:9,netherstar:10,tnt:11};
 const index=category[icon]||({Fabric:2,Forge:3,NeoForge:4,Quilt:5})[loader]||1;
 return <GraphiteIcon id={key('categories',index)} size={38}/>;
}
export function VersionIcon({type,...props}){
 return <GraphiteIcon {...props} id={key('categories',type==='snapshot'?13:['old_alpha','old_beta'].includes(type)?14:1)}/>;
}
export function ContentIcon({kind,...props}){
 return <GraphiteIcon {...props} id={key('actions',{mod:12,modpack:13,resourcepack:14,shader:15}[kind]||13)}/>;
}
export function ProjectIcon({src,kind}){
 const [failed,setFailed]=useState(null),[loaded,setLoaded]=useState(null);
 return <span className="catalog-project-icon">{(!src||failed===src||loaded!==src)&&<ContentIcon kind={kind} size={30}/>} {src&&failed!==src&&<img src={src} alt="" loading="lazy" style={{opacity:loaded===src?1:0}} onLoad={()=>setLoaded(src)} onError={()=>setFailed(src)}/>}</span>;
}
export function AccountIcon({type,...props}){
 return <GraphiteIcon {...props} id={key('actions',type==='offline'?29:17)}/>;
}
const glyph=(sheet,index)=>function Icon(props){return <GraphiteIcon {...props} id={key(sheet,index)}/>};
const utility=(Base,id)=>function Icon(props){const theme=useTheme();return theme==='smoky'?<GraphiteIcon {...props} id={id}/>:<Base {...props}/>};
export const Play=utility(BasePlay,key('actions',1)),Check=utility(BaseCheck,key('states',25)),WarningCircle=utility(BaseWarning,key('states',26));
export const ClockCounterClockwise=utility(BaseHistory,key('states',5));
export const DownloadSimple=glyph('actions',5),ArrowClockwise=glyph('actions',6),FolderOpen=glyph('actions',9),SquaresFour=glyph('actions',10),PuzzlePiece=glyph('actions',12),Package=glyph('actions',13),Stack=glyph('actions',14),Globe=glyph('actions',16),UserCircle=glyph('actions',17),GearSix=glyph('actions',18),SlidersHorizontal=glyph('actions',20),FloppyDisk=glyph('actions',21),Copy=glyph('actions',23),PencilSimple=glyph('actions',24),Trash=glyph('actions',25),CornersOut=glyph('actions',28);
export const Cube=glyph('categories',1),CubeFocus=glyph('categories',1);
export const ImportIcon=glyph('actions',7),NewProfile=glyph('actions',11),LogsIcon=glyph('actions',26),JavaIcon=glyph('actions',19),MemoryIcon=glyph('actions',20),ShaderIcon=glyph('actions',15);
export const VersionsIcon=glyph('categories',1),RefreshIcon=glyph('actions',30);
