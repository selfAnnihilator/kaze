import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
async function module(path, replace = s => s) {
 const source = replace(await readFile(new URL(path, import.meta.url), 'utf8'));
 return import('data:text/javascript;base64,' + Buffer.from(ts.transpile(source, {module:ts.ModuleKind.ESNext, target:ts.ScriptTarget.ES2022})).toString('base64'));
}
const {ownBackendSubscription} = await module('../src/services/backendSubscription.ts');
const tick = () => new Promise(resolve => setImmediate(resolve));
const originalSet = globalThis.setTimeout, originalClear = globalThis.clearTimeout;
const timers = new Map();let seq=0, waits=[];
globalThis.setTimeout = (fn, delay) => { waits.push(delay); timers.set(++seq, fn); return seq; };
globalThis.clearTimeout = id => timers.delete(id);
try {
 let attempts=0, active=0, refreshed=0, failed=false;
 const owner=ownBackendSubscription(async()=>{if(++attempts<=4)throw Error('offline');active++;return()=>active--;},()=>refreshed++,v=>failed=v);
 await tick();for(let i=0;i<3;i++){const [id,fn]=timers.entries().next().value;timers.delete(id);fn();await tick();}
 assert.deepEqual(waits,[1000,2000,4000]);assert.equal(failed,true);assert.equal(active,0);
 owner.retry();owner.retry();await tick();assert.equal(active,1);assert.equal(refreshed,1);assert.equal(failed,false);owner.dispose();assert.equal(active,0);
 let resolve;const late=ownBackendSubscription(()=>new Promise(r=>resolve=r),()=>assert.fail('late refresh'),()=>{});
 late.dispose();resolve(()=>active--);active=1;await tick();assert.equal(active,0);
 const cancelled=ownBackendSubscription(async()=>{throw Error('offline');},()=>{},()=>{});await tick();cancelled.dispose();assert.equal(timers.size,0);
 console.log('PASS subscription backoff, exhaustion, retry ownership, late registration, teardown');
} finally {globalThis.setTimeout=originalSet;globalThis.clearTimeout=originalClear;}
const jobs=[];
globalThis.__artQuery=()=>new Promise(resolve=>jobs.push(resolve));
const art=await module('../src/services/coverArt.ts',s=>s.replace('import { executeQuery } from "./api";', 'const executeQuery = globalThis.__artQuery;'));
const first=art.loadCoverArt('one'), duplicate=art.loadCoverArt('one');assert.equal(jobs.length,1);
art.clearCoverArtCache();assert.equal(await first,null);assert.equal(await duplicate,null);
const current=art.loadCoverArt('one');assert.equal(jobs.length,2);jobs[1]({data:'blue'});assert.equal(await current,'blue');jobs[0]({data:'red'});await tick();assert.equal(art.peekCoverArt('one'),'blue');
const cached = art.loadCoverArt('one');art.clearCoverArtCache();assert.equal(await cached,null);assert.equal(art.peekCoverArt('one'),undefined);
const controller=new AbortController();const abandoned=art.loadCoverArt('abandoned',controller.signal);controller.abort();assert.equal(await abandoned,null);
for(let i=0;i<8;i++)void art.loadCoverArt('limit'+i);
assert.equal(jobs.length,6,'four active requests, including abandoned dispatched IPC');
for(let i=2;i<jobs.length;i++){jobs[i]({data:'ok'});await tick();}
console.log('PASS artwork deduplication, cached/in-flight invalidation, old-last completion, detach, concurrency');
