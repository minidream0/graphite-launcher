import * as THREE from 'three';
import {RoomEnvironment} from 'three/addons/environments/RoomEnvironment.js';

const clamp=t=>Math.min(1,Math.max(0,t));
const ease=t=>1-(1-clamp(t))**4;
// Front silhouettes use the same 256-unit coordinates as public/graphite.svg.
const pieces=[
 {points:[[26,56],[58,24],[143,24],[143,86],[93,86],[81,99],[81,153],[26,153]],face:'#c3beb6',side:'#69635e',offset:[-1.55,.9,1.4],rotation:[-.12,-.5,-.15]},
 {points:[[147,25],[227,25],[235,34],[235,115],[147,115]],face:'#545153',side:'#343134',offset:[1.8,1.45,-.7],rotation:[.28,.7,.24]},
 {points:[[26,165],[81,165],[113,190],[113,231],[34,231],[26,223]],face:'#625c5e',side:'#373236',offset:[-1.25,-1.65,.4],rotation:[-.4,-.65,-.25]},
 {points:[[139,151],[219,151],[219,209],[198,231],[139,231]],face:'#bab3aa',side:'#655e56',offset:[1.6,-1.1,1.25],rotation:[.35,.6,.16]},
 {points:[[81,153],[101,153],[136,179],[110,179]],face:'#eb947c',side:'#b46551',offset:[-.5,-.65,.2],rotation:[0,-.3,-.1],seam:true},
];

export function createStartupScene(canvas){
 let renderer,environment,texture,shadow;
 const scene=new THREE.Scene(),group=new THREE.Group(),materials=[],geometries=[],parts=[];
 let disposed=false;
 function dispose(){
  if(disposed)return;disposed=true;
  for(const g of geometries)g.dispose();for(const m of materials)m.dispose();
  texture?.dispose();environment?.dispose();shadow?.dispose();
  renderer?.dispose();renderer?.forceContextLoss();
 }
 try{
  renderer=new THREE.WebGLRenderer({canvas,alpha:true,antialias:true,powerPreference:'default'});
  renderer.setClearColor(0x000000,0);renderer.outputColorSpace=THREE.SRGBColorSpace;
  renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=.86;
  renderer.shadowMap.enabled=true;renderer.shadowMap.type=THREE.VSMShadowMap;
  const room=new RoomEnvironment(),pmrem=new THREE.PMREMGenerator(renderer);
  try{environment=pmrem.fromScene(room,.06);scene.environment=environment.texture;scene.environmentIntensity=.65;}
  finally{room.dispose();pmrem.dispose()}
  const noise=new Uint8Array(128*128);
  for(let i=0;i<noise.length;i++)noise[i]=105+((i*73^(i>>3)*151)%45);
  texture=new THREE.DataTexture(noise,128,128,THREE.RedFormat);texture.wrapS=texture.wrapT=THREE.RepeatWrapping;texture.repeat.set(3,3);texture.needsUpdate=true;
  for(const [index,piece] of pieces.entries()){
   const shape=new THREE.Shape();piece.points.forEach(([x,y],i)=>{shape[i?'lineTo':'moveTo']((x-128)/64,(128-y)/64)});shape.closePath();
   const depth=piece.seam?.25:.44;
   const geometry=new THREE.ExtrudeGeometry(shape,{depth,steps:1,bevelEnabled:true,bevelSize:piece.seam?.012:.035,bevelThickness:.035,bevelSegments:3,curveSegments:1});
   geometries.push(geometry);geometry.computeBoundingBox();
   const center=geometry.boundingBox.getCenter(new THREE.Vector3());geometry.translate(-center.x,-center.y,-center.z);
   const face=new THREE.MeshStandardMaterial({color:piece.face,metalness:piece.seam?.35:.38,roughness:piece.seam?.35:.49,bumpMap:texture,bumpScale:.006,emissive:piece.seam?'#b94f35':'#000000',emissiveIntensity:0});
   const side=new THREE.MeshStandardMaterial({color:piece.side,metalness:.48,roughness:.38,bumpMap:texture,bumpScale:.006});materials.push(face,side);
   const mesh=new THREE.Mesh(geometry,[face,side]);mesh.castShadow=true;mesh.receiveShadow=true;group.add(mesh);
   parts.push({mesh,face,base:new THREE.Vector3(center.x,center.y,piece.seam?.04:0),offset:new THREE.Vector3(...piece.offset),rotation:piece.rotation,index,seam:piece.seam});
  }
  scene.add(group);
  const floorGeometry=new THREE.PlaneGeometry(50,30),floorMaterial=new THREE.ShadowMaterial({opacity:.17,depthWrite:false});geometries.push(floorGeometry);materials.push(floorMaterial);
  const floor=new THREE.Mesh(floorGeometry,floorMaterial);floor.position.z=-.43;floor.receiveShadow=true;scene.add(floor);
  scene.add(new THREE.HemisphereLight('#e8dace','#29242a',.7));
  const key=new THREE.DirectionalLight('#ffe9d6',2.5);key.position.set(-4,5,7);key.castShadow=true;
  shadow=key.shadow;key.shadow.mapSize.set(512,512);key.shadow.blurSamples=6;key.shadow.radius=3;key.shadow.camera.left=-7;key.shadow.camera.right=7;key.shadow.camera.top=7;key.shadow.camera.bottom=-7;key.shadow.normalBias=.025;key.shadow.bias=-.00015;scene.add(key);
  const fill=new THREE.DirectionalLight('#e1ddd9',1.1);fill.position.set(5,-2,5);scene.add(fill);
  const seamLight=new THREE.PointLight('#ee957c',0,8,2);seamLight.position.set(-.1,-.55,1.1);scene.add(seamLight);
  const camera=new THREE.PerspectiveCamera(35,1,.1,80),target=new THREE.Vector3();
  let width=1,height=1;
  function resize(){
   width=Math.max(1,canvas.clientWidth);height=Math.max(1,canvas.clientHeight);
   renderer.setPixelRatio(Math.min(devicePixelRatio||1,1.5,Math.sqrt(2000000/(width*height))));
   renderer.setSize(width,height,false);camera.aspect=width/height;camera.updateProjectionMatrix();
  }
  resize();
  function frame(seconds){
   if(disposed)return;
   const pull=ease(seconds/1.6),align=ease((seconds-2.25)/.4);
   camera.position.set(THREE.MathUtils.lerp(2.5,0,pull),THREE.MathUtils.lerp(1.6,0,pull),THREE.MathUtils.lerp(4.7,17.5,pull));
   target.set(THREE.MathUtils.lerp(-1,0,pull),THREE.MathUtils.lerp(.85,0,pull),0);camera.lookAt(target);
   group.rotation.set(THREE.MathUtils.lerp(.1,0,align),THREE.MathUtils.lerp(-.28,0,align),0);
   for(const p of parts){
    const progress=ease((seconds-(p.seam?1.25:.48+p.index*.09))/(p.seam?.65:1.15));
    p.mesh.position.copy(p.base).addScaledVector(p.offset,1-progress);
    p.mesh.rotation.set(...p.rotation.map(r=>r*(1-progress)));
    if(p.seam)p.face.emissiveIntensity=.15+Math.sin(Math.PI*clamp((seconds-1.62)/.65))*.9;
   }
   const impact=clamp((seconds-1.85)/.24);group.scale.setScalar(1+Math.sin(impact*Math.PI)*.012);
   seamLight.intensity=.2+Math.sin(Math.PI*clamp((seconds-1.62)/.75))*2.6;
   renderer.render(scene,camera);
  }
  frame(0);
  return {frame,resize,dispose,flightBounds(){const size=4*height/(2*Math.tan(THREE.MathUtils.degToRad(35/2))*17.5);return {size,width,height}},info(){return {calls:renderer.info.render.calls,triangles:renderer.info.render.triangles,pixels:canvas.width*canvas.height}}};
 }catch(error){dispose();throw error}
}
