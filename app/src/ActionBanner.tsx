import { useEffect, useRef, useState } from 'react';
import { Button } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { zh } from './i18n/zh-CN';
import type { ActionItem } from './types';
export function ActionBanner({ items, focus, done, onError, view }: { items: ActionItem[]; focus: number; done: () => Promise<void>; onError: (e: string) => void; view: (item: ActionItem) => void }) {
  const [expanded,setExpanded]=useState(false);
  const [busy,setBusy]=useState(false);
  const root=useRef<HTMLElement>(null);
  useEffect(()=>{if(focus)root.current?.focus();},[focus]);
  async function resolve(item: ActionItem) {
    setBusy(true);
    try { await invoke('resolve_action',{project:item.project,kind:item.kind,id:item.id});await done(); }
    catch(e){onError(String(e));}finally{setBusy(false);}
  }
  if(!items.length)return null;
  return <section className="action-banner" aria-label={zh.actionTitle} tabIndex={-1} ref={root}>
    <div className="card-heading"><strong>{zh.actionCount.replace('{count}',String(items.length))}</strong>{items.length>1&&<Button appearance="subtle" onClick={()=>setExpanded(!expanded)}>{expanded?zh.actionCollapse:zh.actionExpand}</Button>}</div>
    <div className="action-items">{(expanded?items:items.slice(0,1)).map(item=><div className="action-item" key={`${item.kind}:${item.id}`}>
      <p><strong>{zh.actionWaiting.replace('{agent}',item.agent)}</strong>{item.content}</p>
      <div className="button-row"><Button disabled={busy} onClick={()=>void resolve(item)}>{item.kind==='message'?zh.actionDone:zh.actionKnown}</Button><Button onClick={()=>view(item)}>{zh.actionView}</Button></div>
    </div>)}</div>
  </section>;
}
