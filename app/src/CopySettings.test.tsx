// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { CopySettings } from './CopySettings';
import { zh } from './i18n/zh-CN';
const api = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => api);
it('existing copy remains off until explicit enable, ignore checkbox controls file authorization', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const host=document.createElement('div');document.body.append(host);const root=createRoot(host);
  let enabled=false;
  api.invoke.mockReset().mockImplementation(async (command,args) => {
    if(command==='save_copy_settings') enabled=args.enabled;
    if(command==='copy_settings') return {enabled,git:true,ignored:false,error:'write failed',warning:null,last_event_id:0};
  });
  try {
    await act(async()=>root.render(<CopySettings project="/example" refresh={0} done={async()=>{}} onError={vi.fn()}/>));
    expect(api.invoke.mock.calls.every(([c])=>c==='copy_settings')).toBe(true);
    expect(host.textContent).toContain(zh.copyPrivacy);
    expect(host.textContent).toContain('write failed');
    await act(async()=>host.querySelector<HTMLInputElement>('input[type=checkbox]:not([role=switch])')!.click());
    await act(async()=>host.querySelector<HTMLInputElement>('input[role=switch]')!.click());
    expect(api.invoke).toHaveBeenCalledWith('save_copy_settings',{project:'/example',enabled:true,addIgnore:false});
    const button=[...host.querySelectorAll('button')].find(b=>b.textContent===zh.copyIgnoreButton)!;
    await act(async()=>button.click());
    expect(api.invoke).toHaveBeenCalledWith('ignore_record_copy',{project:'/example'});
  } finally { await act(async()=>root.unmount());host.remove();vi.unstubAllGlobals(); }
});
