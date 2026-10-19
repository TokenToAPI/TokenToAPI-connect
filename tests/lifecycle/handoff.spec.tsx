import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
const native = vi.hoisted(() => ({ pending: null as any, listener: () => {}, accept: vi.fn(), discover: vi.fn(), restore: vi.fn(), points: vi.fn() }));
vi.mock('../../src/native', () => ({
 nativeAvailable: true, readStatus: async () => ({sandbox:true,clients:[{client:'codex',hasBackup:true}]}),
 readRestorePoints: native.points, restoreConfig: native.restore,
 pendingHandoff: async () => native.pending,
 watchHandoffs: async (callback: () => void) => {native.listener=callback; return () => {};},
 acceptHandoff: native.accept, discoverHandoffModels:native.discover,
 cancelHandoff: vi.fn(), discoverModels:vi.fn(), importConfig:vi.fn(),
}));
import App from '../../src/App';
let root:Root; let host:HTMLDivElement;
const settle=async () => {await act(async () => {await new Promise(resolve=>setTimeout(resolve,0));});};
const click=async (text:string) => {const button=[...host.querySelectorAll('button')].find(b=>(b.querySelector('strong')?.textContent ?? b.textContent)?.trim()===text)!; expect(button).toBeTruthy(); await act(async()=>button.click()); await settle();};
beforeEach(async () => {
 (globalThis as any).IS_REACT_ACT_ENVIRONMENT=true;
 HTMLDialogElement.prototype.showModal=function(){this.open=true;}; HTMLDialogElement.prototype.close=function(){this.open=false;};
 window.scrollTo=vi.fn(); vi.stubGlobal('localStorage', {clear:vi.fn(),getItem:()=>null,setItem:vi.fn()}); native.pending=null; vi.clearAllMocks();
 native.points.mockResolvedValue([{client:'codex',available:true,reviewId:'review',files:[{modifiedSinceImport:true}],error:null}]);
 native.accept.mockResolvedValue({requestId:'handoff',providerName:'Fixture',baseUrl:'https://tokentoapi.com',suggestedClient:'codex',allowedClients:['codex','codex-desktop'],gatewayNeedsReview:false});
 native.discover.mockResolvedValue({discoveryId:'models',models:[{id:'gpt-5.5',name:'GPT'}]});
 host=document.createElement('div');document.body.append(host);root=createRoot(host); await act(async()=>root.render(<App/>)); await settle();
});
afterEach(async()=>{await act(async()=>root.unmount());host.remove();});
it('queues a known-platform import throughout restore review and an in-flight restore',async()=>{
 await click('Use original config');
 expect(host.querySelector('[aria-labelledby="restore-title"]')).toBeTruthy();
 native.pending={requestId:'handoff',issuer:'https://tokentoapi.com',knownIssuer:true}; await act(async()=>native.listener()); await settle();
 expect(native.accept).not.toHaveBeenCalled();
 expect(host.querySelector('[aria-labelledby="handoff-title"]')).toBeNull();
 let finish!:(value:any)=>void; native.restore.mockImplementation(()=>new Promise(resolve=>finish=resolve));
 const check=host.querySelector('.restore-conflict input') as HTMLInputElement; await act(async()=>check.click());
 await click('Restore'); expect(native.restore).toHaveBeenCalledTimes(1); expect(native.accept).not.toHaveBeenCalled();
 await act(async()=>finish({client:'codex',wasPending:false})); await settle(); expect(native.accept).not.toHaveBeenCalled();
 await click('Done'); await settle(); expect(native.accept).toHaveBeenCalledTimes(1); expect(native.discover).toHaveBeenCalledTimes(1);
 expect(host.textContent).toContain('Platform import · Fixture');
});
it.each([
 {allowed:['codex','codex-desktop','claude','claude-vscode','claude-desktop'],suggested:'codex',options:5},
 {allowed:['claude','claude-vscode'],suggested:'claude',options:2},
])('offers only authoritative clients and fetches models for the chosen client: $suggested',async({allowed,suggested,options})=>{
 native.accept.mockResolvedValue({requestId:'handoff',providerName:'Fixture',baseUrl:'https://tokentoapi.com',suggestedClient:suggested,allowedClients:allowed,gatewayNeedsReview:false});
 native.pending={requestId:'handoff',issuer:'https://tokentoapi.com',knownIssuer:true}; await act(async()=>native.listener()); await settle();
 const select=host.querySelector('[aria-label="Client for platform import"]') as HTMLSelectElement;
 expect(select.options).toHaveLength(options);expect([...select.options].map(option=>option.value)).toEqual(expect.arrayContaining(allowed));
 await act(async()=>{select.value='claude-vscode';select.dispatchEvent(new Event('change',{bubbles:true}));});await settle();
 expect(native.discover).toHaveBeenLastCalledWith('handoff','claude-vscode');
});
