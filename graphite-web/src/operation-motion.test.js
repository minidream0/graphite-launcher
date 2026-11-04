import {test} from 'node:test';
import assert from 'node:assert/strict';
import {Box3} from 'three';
import {createOperationModel} from './operation-scene.js';
import {createMotionClock} from './operations.js';

test('visual clock resumes after hiding and bounds frame jumps without restarting',()=>{
 const clock=createMotionClock();assert.equal(clock.step(0),0);assert.equal(clock.step(16),.016);
 clock.pause();assert.equal(clock.step(90000),.016);assert.equal(clock.step(90016),.032);
 assert(Math.abs(clock.step(95000)-.082)<1e-9);assert(clock.elapsed>0);
});
test('visible portal frames and server pulses do not jump at any recycle boundary over five minutes',()=>{
 for(const kind of ['portal','route']){
  const model=createOperationModel(kind),tracked=model.root.children.filter(m=>m.name===(kind==='portal'?'portal-frame':'route-pulse'));
  const previous=new Map();
  try{
   for(let frame=0;frame<=300*60;frame++){
    model.update(frame/60);
    for(const mesh of tracked){
     const opacity=mesh.isMesh?mesh.material.opacity:mesh.children[0].material.opacity;
     const old=previous.get(mesh);if(old&&mesh.visible&&old.visible)assert(mesh.position.distanceTo(old.position)<.06,`${kind}: visible teleport at ${frame/60}s`);
     assert(opacity>=0&&opacity<=1);previous.set(mesh,{position:mesh.position.clone(),visible:mesh.visible});
    }
   }
  }finally{model.dispose()}
 }
});
test('assembly, mod, scanner and update solids keep clearance throughout intro, loops and completion',()=>{
 for(const kind of ['assembly','mods','scan','update']){
  const model=createOperationModel(kind);
  try{
   for(let frame=0;frame<=1200;frame++){
    const t=frame/4;model.update(t,frame>=1192?{ended:1,state:'succeeded'}:null);
    // Inspect in model space: rotating the whole object must not create false AABB intersections.
    model.root.rotation.set(0,0,0);model.root.updateMatrixWorld(true);
    const solids=[];model.root.traverse(mesh=>{if(mesh.isMesh&&['plate','spine','shell','core'].includes(mesh.name))solids.push(mesh)});
    const bounds=solids.map(mesh=>new Box3().setFromObject(mesh));
    for(let i=0;i<bounds.length;i++)for(let j=i+1;j<bounds.length;j++)assert(!bounds[i].intersectsBox(bounds[j]),`${kind}: ${solids[i].name}/${solids[j].name} overlap at ${t}s`);
    if(kind==='scan'&&t>=1.3){const beam=model.root.getObjectByName('scan-beam'),b=new Box3().setFromObject(beam);for(const bound of bounds)assert(!bound.intersectsBox(b),'scanner intersects a solid')}
   }
   model.root.traverse(mesh=>{if(mesh.isMesh)for(const n of mesh.geometry.attributes.normal.array)assert(Number.isFinite(n))});
  }finally{model.dispose()}
 }
});
test('completion begins from the current pose instead of snapping to an endpoint',()=>{
 for(const kind of ['portal','route','assembly','mods','scan','update']){
  const model=createOperationModel(kind);model.update(12);
  const points=[];model.root.traverse(mesh=>{if(mesh.isMesh)points.push({mesh,before:mesh.position.clone()})});
  model.update(12,{ended:1,state:'succeeded'});for(const {mesh,before} of points)assert(mesh.position.distanceTo(before)<1e-8,kind);
  model.update(12+1/60,{ended:1,state:'succeeded'});for(const {mesh,before} of points)assert(mesh.position.distanceTo(before)<.08,kind);
  model.dispose();
 }
});
