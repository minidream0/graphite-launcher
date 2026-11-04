import * as THREE from 'three';
import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {RoomEnvironment} from 'three/addons/environments/RoomEnvironment.js';

const smooth=x=>{const t=Math.min(1,Math.max(0,x));return t*t*(3-2*t)};
const stopDistance=t=>t<1?t-t*t*t+.5*t*t*t*t:.5;

// Independent of WebGL so clearances and complete loop boundaries can be checked directly.
export function createOperationModel(kind){
 const root=new THREE.Group(),geometries=new Map(),materials=new Set(),parts=[];
 let endedAt=null,disposed=false,scanner,pulse,curve,core;
 const material=(color,extra={})=>{const m=new THREE.MeshStandardMaterial({color,roughness:.62,metalness:.32,...extra});materials.add(m);return m};
 const graphite=material('#55504d'),pale=material('#b8afa4'),coral=material('#df977b',{emissive:'#b75434',emissiveIntensity:.4}),dark=material('#332e2b');
 const clone=m=>{const copy=m.clone();materials.add(copy);return copy};
 function box(name,w,h,d,m,parent=root){
  const key=[w,h,d].join(',');let g=geometries.get(key);
  if(!g){g=new RoundedBoxGeometry(w,h,d,2,Math.min(.035,w/5,h/5,d/5));geometries.set(key,g)}
  const mesh=new THREE.Mesh(g,m);mesh.name=name;parent.add(mesh);return mesh;
 }
 function dispose(){if(disposed)return;disposed=true;for(const g of geometries.values())g.dispose();for(const m of materials)m.dispose()}
 try{
  if(kind==='portal'){
   for(let i=0;i<7;i++){
    const frame=new THREE.Group();frame.name='portal-frame';const face=clone(i%3===0?pale:graphite),joint=clone(coral);
    for(const m of [face,joint]){m.transparent=true;m.depthWrite=false}
    for(const [w,h,x,y] of [[3.18,.15,0,1.4],[3.18,.15,0,-1.4],[.15,2.59,-1.51,0],[.15,2.59,1.51,0]])box('frame-bar',w,h,.22,face,frame).position.set(x,y,0);
    box('frame-inlay',.48,.028,.012,joint,frame).position.set(-.8,1.4,.128);
    root.add(frame);parts.push({mesh:frame,index:i,materials:[face,joint]});
   }
  }else if(kind==='route'){
   for(const x of [-1.9,1.9])for(let i=0;i<3;i++){
    box('server',.7,.34,.68,i===2?pale:graphite).position.set(x,(i-1)*.43,0);
    box('server-inlay',.18,.025,.012,coral).position.set(x-.16,(i-1)*.43,.362);
   }
   curve=new THREE.CatmullRomCurve3([new THREE.Vector3(-1.47,0,.12),new THREE.Vector3(-.55,-.35,.42),new THREE.Vector3(.55,.35,.42),new THREE.Vector3(1.47,0,.12)]);
   const geometry=new THREE.TubeGeometry(curve,64,.012,6,false);geometries.set('route',geometry);root.add(new THREE.Mesh(geometry,dark));
   const m=clone(coral);m.transparent=true;m.depthWrite=false;pulse=box('route-pulse',.13,.13,.13,m);
  }else if(kind==='update'){
   for(const [w,h,x,y] of [[.25,1.8,-.84,0],[.25,1.8,.84,0],[1.4,.25,0,.77],[1.4,.25,0,-.77]]){
    const mesh=box('shell',w,h,1.05,parts.length%2?pale:graphite);parts.push({mesh,x,y});
   }
   core=box('core',.65,.65,.65,coral);
  }else{
   for(let i=0;i<6;i++){
    const x=(i%2-.5)*1.04,y=(Math.floor(i/2)-1)*.64;
    parts.push({mesh:box('plate',.84,.49,.84,i%3===0?pale:graphite),x,y,index:i});
   }
   box('spine',.06,1.69,.55,coral);
   if(kind==='scan'){
    scanner=new THREE.Group();scanner.name='scanner';root.add(scanner);
    box('scan-beam',2.1,.019,.018,coral,scanner).position.z=.535;
    const glow=clone(coral);glow.transparent=true;glow.depthWrite=false;glow.opacity=.09;
    box('scan-halo',2.15,.085,.006,glow,scanner).position.z=.52;
   }
  }
  function update(t,item){
   if(item?.ended&&endedAt===null)endedAt=t;
   const finish=endedAt===null?0:Math.max(0,t-endedAt),settle=endedAt===null?0:smooth(finish/1.1);
   const phase=endedAt===null?t:endedAt+stopDistance(finish),intro=smooth(t/1.3);
   root.scale.setScalar(.96+.04*intro);
   if(kind==='portal'){
    // Recycle only while fully transparent, never jump a visible frame back into the tunnel.
    for(const p of parts){
     const z=-18+((p.index*3.6+phase*.7)%25.2),opacity=smooth((z+18)/3.5)*(1-smooth((z-4.5)/2.7));
     p.mesh.position.z=z;p.mesh.visible=opacity>.001;
     for(const m of p.materials){m.opacity=opacity;m.depthWrite=opacity>.985}
    }
    root.rotation.set(.035,-.07+Math.sin(phase*.12)*.018,.015);root.position.y=0;
   }else if(kind==='route'){
    const u=(phase/5.8)%1;pulse.position.copy(curve.getPointAt(u));
    pulse.material.opacity=smooth(u/.12)*(1-smooth((u-.88)/.12));pulse.visible=pulse.material.opacity>.001;
    root.rotation.set(.18,-.2+Math.sin(phase*.3)*.035,-.03);
   }else if(kind==='update'){
    const insert=smooth(t/1.5);
    for(const p of parts)p.mesh.position.set(p.x*(1+.22*(1-insert)),p.y*(1+.22*(1-insert)),0);
    core.position.z=1.55*(1-insert);core.rotation.y=.5*(1-insert);
    root.rotation.set(.28,-.48+Math.sin(phase*.3)*.025,-.04);
   }else{
    for(const p of parts){
     const wave=kind==='mods'?.5-.5*Math.cos(phase*.9-p.index*.65):.5+.5*Math.sin(phase*.65);
     const spread=(1-intro)*.55+(kind==='mods'?.065:.025)*wave*(1-settle);
     p.mesh.position.set(p.x*(1+spread),p.y*(1+spread),(1-intro)*(.16+p.index*.06)+(1-settle)*Math.sin(phase*.7+p.index*.6)*.018);
    }
    if(scanner){scanner.position.y=Math.sin(phase*.85)*.91;scanner.scale.x=smooth((t-1.3)/.4);scanner.visible=t>=1.3}
    root.rotation.set(.28,-.48+Math.sin(phase*.24)*.045,-.04);
   }
  }
  update(0);return {root,update,dispose};
 }catch(error){dispose();throw error}
}

export function createOperationScene(canvas,kind){
 let renderer,environment,model,disposed=false;
 function dispose(){if(disposed)return;disposed=true;model?.dispose();environment?.dispose();renderer?.dispose();renderer?.forceContextLoss()}
 try{
  renderer=new THREE.WebGLRenderer({canvas,alpha:true,antialias:true,powerPreference:'low-power'});
  renderer.setClearColor(0,0);renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=.86;
  const scene=new THREE.Scene(),camera=new THREE.PerspectiveCamera(38,1,.5,50);
  const room=new RoomEnvironment(),pmrem=new THREE.PMREMGenerator(renderer);
  try{environment=pmrem.fromScene(room,.08);scene.environment=environment.texture;scene.environmentIntensity=.65}finally{room.dispose();pmrem.dispose()}
  model=createOperationModel(kind);scene.add(model.root);scene.add(new THREE.HemisphereLight('#f2dcca','#27232b',.9));
  const key=new THREE.DirectionalLight('#ffead7',2.3);key.position.set(-3,5,6);scene.add(key);
  const fill=new THREE.DirectionalLight('#d7d0c7',.9);fill.position.set(4,1,-3);scene.add(fill);
  const light=new THREE.PointLight('#e89d7f',3.5,8,2);light.position.set(0,.7,2);scene.add(light);
  camera.position.z=kind==='portal'?10:6.3;
  function resize(){const w=Math.max(1,canvas.clientWidth),h=Math.max(1,canvas.clientHeight);renderer.setPixelRatio(Math.min(devicePixelRatio||1,1.5,Math.sqrt(700000/(w*h))));renderer.setSize(w,h,false);camera.aspect=w/h;camera.updateProjectionMatrix()}
  function frame(t,item){if(disposed)return;model.update(t,item);renderer.render(scene,camera)}
  resize();return {frame,resize,dispose};
 }catch(error){dispose();throw error}
}
