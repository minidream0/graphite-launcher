import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const catalog=JSON.parse(readFileSync(new URL('./icon-catalog.json',import.meta.url)));
test('all 80 selected icons have distinct native keys and frames inside their source sheet',()=>{
 const keys=new Set();let count=0;
 for(const sheet of catalog.sheets){assert.equal(sheet.labels.length,sheet.frames.length);sheet.frames.forEach((frame,i)=>{assert.equal(frame.length,4);const [x,y,width,height]=frame;assert(x>=0&&y>=0&&width>0&&height>0);assert(x+width<=sheet.width&&y+height<=sheet.height);const key=`graphite-v1-${sheet.id}-${i+1}`;assert(!keys.has(key));keys.add(key);count++;});}
 assert.equal(count,80);
});
test('all 80 smoky icons map to existing meanings and real PNG bounds',()=>{
 const smoky=JSON.parse(readFileSync(new URL('./smoky-icon-catalog.json',import.meta.url))),expected=new Set(catalog.sheets.flatMap(s=>s.frames.map((_,i)=>`graphite-v1-${s.id}-${i+1}`))),actual=new Set();
 for(const sheet of smoky.sheets){const png=readFileSync(new URL('../public/smoky-icons/'+sheet.file,import.meta.url));assert.equal(png.subarray(1,4).toString(),'PNG');assert.equal(png.readUInt32BE(16),sheet.width);assert.equal(png.readUInt32BE(20),sheet.height);assert.equal(sheet.frames.length,16);for(const [i,[x,y,w,h]] of sheet.frames.entries()){assert(x>=0&&y>=0&&w>0&&h>0&&x+w<=sheet.width&&y+h<=sheet.height);const key=`graphite-v1-${sheet.group}-${i+1+sheet.offset}`;assert(!actual.has(key));actual.add(key)}}
 assert.deepEqual(actual,expected);
});
