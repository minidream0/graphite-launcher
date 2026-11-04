import {test} from 'node:test';
import assert from 'node:assert/strict';
import {applyTheme,getTheme,initializeTheme,saveTheme} from './theme.js';
test('theme loads from native preferences, normalizes invalid values and retains the old theme on save failure',async()=>{
 globalThis.document={documentElement:{dataset:{}}};
 assert.equal(await initializeTheme({tools:async()=>({theme:'smoky'})}),'smoky');assert.equal(document.documentElement.dataset.theme,'smoky');
 const failure=new Error('Read-only data');await assert.rejects(saveTheme('graphite',{tools:async()=>{throw failure}}),e=>e===failure);assert.equal(getTheme(),'smoky');
 assert.equal(await saveTheme('graphite',{tools:async(method,p)=>{assert.equal(method,'appearance.set');return p}}),'graphite');
 assert.equal(applyTheme('unknown'),'graphite');assert.equal(await initializeTheme({tools:async()=>{throw failure}}),'graphite');
 delete globalThis.document;
});
