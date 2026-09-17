import test from 'node:test';
import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { readFileSync } from 'node:fs';
import { build } from 'esbuild';
const bundle = await build({entryPoints:['worker/src/index.ts'],bundle:true,write:false,platform:'node',format:'esm'});
const { default: worker } = await import(`data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString('base64')}`);

async function setup() {
  const sqlite = new DatabaseSync(':memory:');
  sqlite.exec(readFileSync('worker/schema.sql','utf8'));
  const now = Math.floor(Date.now()/1000);
  const tokens = {};
  for (const user of ['A','B']) {
    sqlite.prepare('INSERT INTO users (id,username,password_hash,created_at) VALUES (?,?,?,?)').run(user,user,'unused',now);
    tokens[user] = user.repeat(40);
    const hash = Buffer.from(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(tokens[user]))).toString('hex');
    sqlite.prepare('INSERT INTO sessions (id,user_id,token_hash,device_id,device_name,client_version,created_at,last_used_at,idle_expires_at,absolute_expires_at) VALUES (?,?,?,?,?,?,?,?,?,?)').run(user,user,hash,`device-${user}`,'test','test',now,now,now+3600,now+7200);
  }
  const DB = {prepare(sql) {
    let values=[];
    return {bind(...args){values=args;return this;},async first(){return sqlite.prepare(sql).get(...values)??null;},async all(){return {results:sqlite.prepare(sql).all(...values)};},async run(){return sqlite.prepare(sql).run(...values);}};
  },async batch(statements){sqlite.exec('BEGIN');try {const result=[];for(const s of statements)result.push(await s.run());sqlite.exec('COMMIT');return result;} catch(e){sqlite.exec('ROLLBACK');throw e;}}};
  async function request(user,payload) {
    const response=await worker.fetch(new Request('https://test/api/sync',{method:payload?'POST':'GET',headers:{Authorization:`Bearer ${tokens[user]}`,'Content-Type':'application/json'},...(payload?{body:JSON.stringify(payload)}:{})}),{DB});
    assert.equal(response.status,200,await response.clone().text());
    return response.json();
  }
  return {sqlite,request};
}
const row=(device,seconds,updated_at=100)=>({user_id:'B',device_id:device,stat_date:'2026-09-17',listening_seconds:seconds,play_count:1,completion_count:1,skip_count:0,updated_at});
test('Worker: duplicate push, stale clocks, two devices, empty push, tenant isolation',async()=>{
  const {sqlite,request}=await setup();
  for(const ds of [row('one',3600),row('one',4200,50),row('one',4200,50),row('one',100,9999999999),row('two',1000)]) await request('A',{daily_stats:[ds]});
  await request('B',{daily_stats:[row('one',99)]});
  await request('A',{daily_stats:[]});
  const a=await request('A'); const b=await request('B');
  assert.equal(a.data.daily_stats.length,2);
  assert.equal(a.data.daily_stats.reduce((s,d)=>s+d.listening_seconds,0),5200);
  assert.ok(a.data.daily_stats.every(d=>d.user_id==='A'));
  assert.equal(b.data.daily_stats.length,1);assert.equal(b.data.daily_stats[0].listening_seconds,99);
  assert.equal(sqlite.prepare("SELECT COUNT(*) AS n FROM daily_user_stats WHERE user_id='A'").get().n,2);
});
test('Audit evidence: lifetime MAX merge loses independent offline contributions',async()=>{
  const {request}=await setup();
  const song_stats=seconds=>[{song_id:'song',total_time_listened:seconds,play_count:1}];
  await request('A',{songs:[{id:'song',title:'Song'}],song_stats:song_stats(100)});
  await request('A',{song_stats:song_stats(130)});
  await request('A',{song_stats:song_stats(140)});
  const data=await request('A');
  assert.equal(data.data.song_stats[0].total_seconds,140); // Correct union would be 170.
});

test('Phase 22 Worker: track_device_stats multi-device accumulation, baseline preservation, repeated push idempotency', async () => {
  const { sqlite, request } = await setup();
  // baseline 100
  await request('A', {
    track_device_stats: [
      { device_id: '_baseline', track_id: 'song-1', play_count: 5, total_seconds: 100, completion_count: 5, skip_count: 0 }
    ]
  });
  // Device A +30
  await request('A', {
    track_device_stats: [
      { device_id: 'device_A', track_id: 'song-1', play_count: 1, total_seconds: 30, completion_count: 1, skip_count: 0 }
    ]
  });
  // Device B +40
  await request('A', {
    track_device_stats: [
      { device_id: 'device_B', track_id: 'song-1', play_count: 2, total_seconds: 40, completion_count: 2, skip_count: 0 }
    ]
  });

  // Pull from Cloud (simulates fresh install restore or regular pull)
  let pull = await request('A');
  assert.ok(pull.data.track_device_stats, 'track_device_stats returned in GET /api/sync');
  assert.equal(pull.data.track_device_stats.length, 3);
  let totalLifetime = pull.data.track_device_stats.reduce((acc, row) => acc + row.total_seconds, 0);
  assert.equal(totalLifetime, 170, 'baseline 100 + A 30 + B 40 = 170');

  // Repeated push: remains 170 (idempotent)
  await request('A', {
    track_device_stats: [
      { device_id: '_baseline', track_id: 'song-1', play_count: 5, total_seconds: 100, completion_count: 5, skip_count: 0 },
      { device_id: 'device_A', track_id: 'song-1', play_count: 1, total_seconds: 30, completion_count: 1, skip_count: 0 },
      { device_id: 'device_B', track_id: 'song-1', play_count: 2, total_seconds: 40, completion_count: 2, skip_count: 0 }
    ]
  });
  pull = await request('A');
  totalLifetime = pull.data.track_device_stats.reduce((acc, row) => acc + row.total_seconds, 0);
  assert.equal(totalLifetime, 170, 'Repeated push remains 170');

  // Updated Device A: A 30 -> 50, expected total 190 (not 220)
  await request('A', {
    track_device_stats: [
      { device_id: 'device_A', track_id: 'song-1', play_count: 2, total_seconds: 50, completion_count: 2, skip_count: 0 }
    ]
  });
  pull = await request('A');
  totalLifetime = pull.data.track_device_stats.reduce((acc, row) => acc + row.total_seconds, 0);
  assert.equal(totalLifetime, 190, 'Updated Device A 30->50 yields exactly 190, not 220');
});

test('Phase 22 Worker: track_device_stats tenant isolation and adversarial user_id spoofing', async () => {
  const { sqlite, request } = await setup();
  // Authenticated User A tries to spoof user_id = 'B'
  await request('A', {
    track_device_stats: [
      { user_id: 'B', device_id: 'device_A', track_id: 'spoofed_track', play_count: 10, total_seconds: 500, completion_count: 10, skip_count: 0 }
    ]
  });

  // Check User A data
  const pullA = await request('A');
  assert.equal(pullA.data.track_device_stats.length, 1);
  assert.equal(pullA.data.track_device_stats[0].track_id, 'spoofed_track');
  assert.equal(pullA.data.track_device_stats[0].total_seconds, 500);

  // Check User B data: MUST be empty (User A row was NOT written to User B)
  const pullB = await request('B');
  assert.equal(pullB.data.track_device_stats.length, 0, 'User B has no rows');

  // Verify in SQLite database directly
  const rowsB = sqlite.prepare("SELECT * FROM track_device_stats WHERE user_id = 'B'").all();
  assert.equal(rowsB.length, 0, 'No rows in DB for User B');

  const rowsA = sqlite.prepare("SELECT * FROM track_device_stats WHERE user_id = 'A'").all();
  assert.equal(rowsA.length, 1, 'Exactly one row for User A');
  assert.equal(rowsA[0].user_id, 'A');
});

