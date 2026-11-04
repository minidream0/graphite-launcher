import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createOperations,gameReady} from './operations.js';
test('RPC observation preserves results, tracks real queue phases and errors',async()=>{
 const s=createOperations();let notified=0;const unsub=s.subscribe(()=>notified++);
 const result={job:'queue-1',queued:true};assert.equal(await s.observe('modrinth',['install',{kind:'mod'}],async()=>result),result);
 assert.equal(s.getSnapshot()[0].kind,'mods');assert.equal(s.getSnapshot()[0].state,'queued');
 await s.observe('engine',['state'],async()=>({jobs:[{id:'queue-1',state:'running',status:'Download',current:2,total:5}]}));
 assert.equal(s.getSnapshot()[0].current,2);assert.equal(s.getSnapshot()[0].ended,undefined);
 await s.observe('engine',['state'],async()=>({jobs:[{id:'queue-1',state:'failed',error:'No disk space'}]}));
 assert.equal(s.getSnapshot()[0].status,'No disk space');assert(s.getSnapshot()[0].ended);
 await s.observe('queue',['retry','queue-1'],async()=>({job:'queue-2',queued:true}));assert.equal(s.getSnapshot()[1].kind,'mods');assert.equal(s.getSnapshot()[1].job,'queue-2');
 const error=new Error('AUTH_REQUIRED');await assert.rejects(s.observe('servers',['launch',{}],async()=>{throw error}),e=>e===error);
 assert.equal(s.getSnapshot()[2].kind,'route');assert.equal(s.getSnapshot()[2].state,'failed');assert(notified>0);unsub();
});
test('all operation paths and fast synchronous preflight completion',async()=>{
 const s=createOperations();
 for(const [channel,args,kind] of [['engine',['launch',{id:'a'}],'portal'],['engine',['create',{}],'assembly'],['servers',['prepare',{}],'assembly'],['import-instance',['token'],'assembly'],['pack-update',['update','a'],'assembly'],['tools',['preflight',{id:'a'}],'scan']]){
  await s.observe(channel,args,async()=>({errors:2}));const item=s.getSnapshot().at(-1);assert.equal(item.kind,kind);assert(item.ended);
 }
 assert.equal(s.getSnapshot().at(-1).status,'Найдены ошибки');
 const count=s.getSnapshot().length;await s.observe('tools',['history'],async()=>[]);assert.equal(s.getSnapshot().length,count);
 await s.observe('modrinth',['install',{kind:'resourcepack'}],async()=>({job:'resource'}));assert.equal(s.getSnapshot().length,count);
 await s.observe('drop-files',[['mod.jar'],'a'],async()=>({kind:'mods',count:1}));assert.equal(s.getSnapshot().at(-1).kind,'mods');assert.equal(s.getSnapshot().at(-1).status,'Добавлено модов: 1');
});
test('game readiness uses renderer log signals, not elapsed time or server availability',()=>{
 assert(gameReady('[12:00:00] [Render thread/INFO]: Sound engine started'));
 assert(gameReady('[Client thread/INFO]: Created: 1024x512 textures-atlas'));
 assert(!gameReady('Downloading libraries…'));assert(!gameReady('Server online'));assert(!gameReady('[Server thread/INFO]: Sound engine started'));
});
