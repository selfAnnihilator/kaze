import assert from 'node:assert/strict';
import {installDualInterfaceFixture} from '../tests/fixtures/dualInterfaceBackend.mjs';
const {chromium}=await import(process.env.KAZE_PLAYWRIGHT_MODULE||'playwright');
const browser=await chromium.launch({executablePath:process.env.KAZE_CHROMIUM||'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
try {for(const style of ['spatial','brutalist']) {
 const context=await browser.newContext();await context.addInitScript({content:`sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();const f=window.__kazeFixture,q=f.query.bind(f),c=f.command.bind(f);f.onboardingDone=false;f.denyFolder=true;f.query=r=>r.query==='GetOnboardingStatus'?Promise.resolve({type:'OnboardingStatus',completed:f.onboardingDone,default_music_dir:'/fixture',configured_folders:[]}):q(r);f.command=async r=>{if(r.command==='CompleteOnboarding'){f.commands.push(r);await new Promise(resolve=>setTimeout(resolve,100));if(f.denyFolder)throw Error('Permission denied: audit folder');f.onboardingDone=true;return{status:'OnboardingCompleted',configured_folders:1};}return c(r);};`});
 await context.route('**/src/services/api.ts*',r=>r.fulfill({contentType:'text/javascript',body:`export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};`}));
 const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(process.env.KAZE_TEST_URL||'http://127.0.0.1:5194');
 await page.getByRole('button',{name:'Confirm & Start Kaze',exact:true}).click();await page.waitForTimeout(350);
 if(process.env.KAZE_REPRO_ONLY){console.log('REPRO',style,{dialog:await page.getByRole('button',{name:'Confirm & Start Kaze',exact:true}).count(),errors});await context.close();continue;}
 await page.getByRole('alert').filter({hasText:'Permission denied'}).waitFor();assert.equal(await page.getByRole('button',{name:'Confirm & Start Kaze',exact:true}).count(),1);
 await page.evaluate(()=>window.__kazeFixture.denyFolder=false);await page.getByRole('button',{name:'Confirm & Start Kaze',exact:true}).click();await page.getByRole('button',{name:'Confirm & Start Kaze',exact:true}).waitFor({state:'detached'});assert.deepEqual(errors,[]);
 console.log('PASS',style,'folder permission failure retains onboarding and successful retry has no unhandled rejection');await context.close();
}}finally{await browser.close();}
