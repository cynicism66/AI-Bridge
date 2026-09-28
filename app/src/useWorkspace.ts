import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Snapshot, Tab } from './types';
import { zh } from './i18n/zh-CN';

export function useWorkspace() {
  const [data, setData] = useState<Snapshot | null>(null);
  const [project, setProject] = useState<string | null>(null);
  const [actionFocus,setActionFocus]=useState(0);
  const [tab, setTab] = useState<Tab>('board');
  const [error, setError] = useState('');
  const [refresh, setRefresh] = useState(0);
  const [busy, setBusy] = useState(false);
  const revision = useRef(-1);
  const select = useCallback((p: string, t: Tab = 'board') => { setProject(p); setTab(t); }, []);
  const reload = useCallback(async () => {
    const next = await invoke<Snapshot>('snapshot');
    revision.current = next.revision;
    setData(next);
    setRefresh(n => n + 1);
    setProject(p => p && [...next.projects, ...next.discovered].some(item => item.project === p) ? p : next.projects[0]?.project || next.discovered[0]?.project || null);
  }, []);
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      let delay = 1500;
      try {
        const status = await invoke<{ revision: number; visible: boolean; target: string | null; targetAction?: boolean; error: string | null }>('check_changes');
        if (!active) return;
        delay = status.visible ? 1500 : 5000;
        if (status.target) {select(status.target, status.targetAction ? 'board' : 'chat'); if(status.targetAction)setActionFocus(n=>n+1);}
        if (status.error) setError(status.error);
        if (status.revision !== revision.current) await reload();
      } catch (e) { if (active) setError(String(e)); }
      finally { if (active) timer = setTimeout(poll, delay); }
    }
    void invoke('window_ready').catch(e => setError(String(e)));
    void poll();
    return () => { active = false; clearTimeout(timer); };
  }, [reload, select]);
  const run = useCallback(async (operation: () => Promise<unknown>) => {
    setBusy(true);
    try { await operation(); await reload(); setError(''); return true; }
    catch (e) { setError(String(e) || zh.operationFailed); return false; }
    finally { setBusy(false); }
  }, [reload]);
  return { actionFocus, data, project, tab, error, busy, refresh, select, setTab, setError, run, reload };
}
