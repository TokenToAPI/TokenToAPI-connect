// Exercises the actual release EXE on an ephemeral Windows CI runner.
import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
if (process.platform !== 'win32' || process.env.GITHUB_ACTIONS !== 'true') throw new Error('Run only on an ephemeral Windows CI runner');
const root = process.cwd();
const stage = path.join(process.env.RUNNER_TEMP, 'Connect Portable Test');
const ps = command => execFileSync('powershell.exe', ['-NoProfile', '-Command', command], { encoding: 'utf8' });
const quote = s => "'" + s.replaceAll("'", "''") + "'";
ps(`Expand-Archive -LiteralPath ${quote(path.join(root, 'release/TokenToAPI-Connect-0.1.4-Windows-x64-portable.zip'))} -DestinationPath ${quote(stage)}`);
const exe = path.join(stage, 'TokenToAPI Connect/TokenToAPI Connect.exe');
process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222';
const delay = ms => new Promise(r => setTimeout(r, ms));
let socket; let proc;
async function until(fn, label) {
  for (let i=0;i<90;i++) { try { const v=await fn(); if(v) return v; } catch {} await delay(1000); }
  throw new Error('Timeout: '+label);
}
async function connect() {
  const page = await until(async()=> (await (await fetch('http://127.0.0.1:9222/json/list')).json()).find(p=>p.type==='page'), 'WebView2 page');
  socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{socket.onopen=resolve;socket.onerror=reject;});
}
let id=0;
async function evaluate(expression) {
  const requestId=++id;
  return await new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('CDP timeout')),10000);
    const listener=event=>{const m=JSON.parse(event.data);if(m.id!==requestId)return;clearTimeout(timer);socket.removeEventListener('message',listener);if(m.error||m.result.exceptionDetails)reject(new Error(JSON.stringify(m)));else resolve(m.result.result.value);};
    socket.addEventListener('message',listener);
    socket.send(JSON.stringify({id:requestId,method:'Runtime.evaluate',params:{expression,returnByValue:true}}));
  });
}
const link = issuer => 'tokentoapi-connect://v1/import?'+new URLSearchParams({issuer,code:'A'.repeat(43)});
const openLink = issuer => ps(`Start-Process ${quote(link(issuer))}`);
const dialog = issuer => until(()=>evaluate(`document.querySelector('dialog[open] .handoff-address')?.textContent === ${JSON.stringify(issuer)}`), 'issuer review '+issuer);
const cancel = async()=> {await evaluate(`Array.from(document.querySelectorAll('dialog[open] button')).find(b=>b.textContent==='Cancel').click()`);await until(()=>evaluate(`!document.querySelector('dialog[open]')`),'cancel');};
const close = ()=> {socket?.close();ps(`Get-Process | Where-Object {$_.Path -eq ${quote(exe)}} | Stop-Process -Force`);};
try {
  proc=spawn(exe,[],{env:process.env,stdio:'ignore'});
  await connect();
  await until(()=>evaluate(`document.body.innerText.includes('Use original config')`),'home render');
  const command=ps(String.raw`(Get-Item 'HKCU:\Software\Classes\tokentoapi-connect\shell\open\command').GetValue('')`).trim();
  assert(command.includes('"'+exe+'"'), 'Protocol must quote the portable EXE path');
  assert(command.includes('%1'), 'Protocol must forward the URL');
  openLink('https://warm.example.test');await dialog('https://warm.example.test');await cancel();
  const count=Number(ps(`@(Get-Process | Where-Object {$_.Path -eq ${quote(exe)}}).Count`).trim());assert.equal(count,1);
  close();await delay(2000);
  openLink('https://cold.example.test');await connect();await dialog('https://cold.example.test');await cancel();
  fs.writeFileSync('release/windows-smoke.json',JSON.stringify({platform:process.platform,arch:process.arch,portableLaunch:true,protocolRegistration:true,warmHandoff:true,coldHandoff:true,singleInstance:true,cancellation:true,configurationWrites:false},null,2));
  console.log('Windows release EXE: portable launch, quoted protocol, cold/warm handoff, single instance, cancellation passed.');
} finally {close();}
