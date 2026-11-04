import {test} from 'node:test';
import assert from 'node:assert/strict';
import {matchingInstances,chooseInstance,normalizeServer,validateServer} from './server-form.js';
const instances=[{id:'new',version:'26.1.2'},{id:'old',version:'1.20.1'},{id:'broken',version:'1.20.1',error:true}];
const draft={name:' Server ',address:' play.example.org ',minecraft:'',project:'',instanceId:''};
test('only name/address required, with explicit field errors and normalization',()=>{
 assert.deepEqual(validateServer(draft,instances),{});assert.equal(normalizeServer(draft).address,'play.example.org');
 assert.deepEqual(Object.keys(validateServer({...draft,name:' ',address:''},instances)),['name','address']);
 for(const address of ['https://example.org','example.org:0','example.org:65536','example.org/path','bad host'])assert(validateServer({...draft,address},instances).address,address);
 assert(validateServer({...draft,project:'https://modrinth.com/modpack/test'},instances).project);
});
test('version chooses a unique matching instance, retains compatible explicit choices, never guesses among packs',()=>{
 assert.equal(chooseInstance(instances,'1.20.1 ','new'),'old');assert.equal(chooseInstance(instances,'',''), '');
 assert.equal(chooseInstance(instances,'1.21.4','new'),'');assert.equal(matchingInstances(instances,'1.20.1').length,1);
 const multiple=[...instances,{id:'second',version:'1.20.1'}];assert.equal(chooseInstance(multiple,'1.20.1','new'),'');assert.equal(chooseInstance(multiple,'1.20.1','second'),'second');
 assert(validateServer({...draft,minecraft:'1.20.1',instanceId:'new'},instances).instanceId);
 assert.deepEqual(validateServer({...draft,minecraft:'1.20.1',instanceId:'old'},instances),{});
});
