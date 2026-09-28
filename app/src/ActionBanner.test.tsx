// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { App } from './App';
import { zh } from './i18n/zh-CN';
import type { ActionItem, Tab } from './types';
const fixture=vi.hoisted(()=>({tab:'board' as Tab,items:[] as ActionItem[],invoke:vi.fn(),select:vi.fn(),reload:vi.fn(async()=>{}),error:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke:fixture.invoke}));
vi.mock('./Board',()=>({Board:()=>null}));
vi.mock('./Chat',()=>({Chat:()=>null}));
vi.mock('./History',()=>({History:()=>null}));
vi.mock('./Settings',()=>({Settings:()=>null}));
vi.mock('./useWorkspace',()=>({useWorkspace:()=>({
  data:{projects:[{project:'d:/test',enabled:true,initialized:true,unread:0}],discovered:[],actions:fixture.items,global:true},
  project:'d:/test',tab:fixture.tab,error:'',busy:false,refresh:0,actionFocus:0,select:fixture.select,setTab:vi.fn(),setError:fixture.error,reload:fixture.reload,run:vi.fn(),
})}));
it('pending banner persists across every tab, expands, navigates and resolves independently of reads',async()=>{
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT',true);
  vi.stubGlobal('matchMedia',()=>({matches:false,addEventListener:vi.fn(),removeEventListener:vi.fn()}));
  fixture.items=[{id:10,kind:'message',agent:'codex',project:'d:/test',content:'decision needed'},{id:20,kind:'blocker',agent:'claude',project:'d:/test',content:'blocked'}];
  fixture.invoke.mockResolvedValue(undefined);
  const host=document.createElement('div');document.body.append(host);const root=createRoot(host);
  async function click(label:string){await act(async()=>[...host.querySelectorAll('button')].find(b=>b.textContent===label)!.click());}
  try{
    for(const tab of ['board','chat','history','settings'] as Tab[]){
      fixture.tab=tab;await act(async()=>root.render(<App/>));
      expect(host.querySelector('.action-banner')!.textContent).toContain('decision needed');
      expect(host.querySelector('.action-banner')!.closest('.tab-content')).toBeNull();
      expect(host.querySelector(`[aria-label="${zh.actionTitle}"]`)).not.toBeNull();
    }
    await click(zh.actionView);expect(fixture.select).toHaveBeenCalledWith('d:/test','chat');
    await click(zh.actionExpand);expect(host.querySelectorAll('.action-item')).toHaveLength(2);
    await click(zh.actionDone);expect(fixture.invoke).toHaveBeenCalledWith('resolve_action',{project:'d:/test',kind:'message',id:10});
    await click(zh.actionKnown);expect(fixture.invoke).toHaveBeenCalledWith('resolve_action',{project:'d:/test',kind:'blocker',id:20});
    fixture.items=[];await act(async()=>root.render(<App/>));expect(host.querySelector('.action-banner')).toBeNull();
  }finally{await act(async()=>root.unmount());host.remove();vi.unstubAllGlobals();}
});
