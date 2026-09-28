import { useEffect, useState } from 'react';
import { Button, Card, Checkbox, Spinner, Switch } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { zh } from './i18n/zh-CN';
export interface CopyState { enabled: boolean; git: boolean; ignored: boolean; warning: string | null; error: string | null; last_event_id: number }
export function CopySettings({ project, refresh, done, onError }: { project: string; refresh: number; done: () => Promise<void>; onError: (e: string) => void }) {
  const [data, setData] = useState<CopyState | null>(null);
  const [addIgnore, setAddIgnore] = useState(true);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    invoke<CopyState>('copy_settings', { project }).then(d => { if (active) setData(d); }).catch(e => { if (active) onError(String(e)); });
    return () => { active = false; };
  }, [project, refresh, onError]);
  async function save(enabled?: boolean) {
    setBusy(true);
    try {
      if (enabled === undefined) await invoke('ignore_record_copy', { project });
      else await invoke('save_copy_settings', { project, enabled, addIgnore: enabled && addIgnore && data?.git && !data.ignored });
      setData(await invoke<CopyState>('copy_settings', { project })); await done();
    } catch (e) { onError(String(e)); } finally { setBusy(false); }
  }
  return <Card><h2>{zh.copyTitle}</h2>{!data ? <Spinner label={zh.loading}/> : <>
    <p className="muted">{zh.copyDescription}</p>
    {data.git && !data.ignored && !data.enabled && <Checkbox label={zh.copyIgnoreConfirm} checked={addIgnore} disabled={busy} onChange={(_,d) => setAddIgnore(d.checked === true)}/>}
    <Switch label={zh.copyEnabled} checked={data.enabled} disabled={busy} onChange={(_,d) => void save(d.checked)}/>
    {data.git && !data.ignored && <><p role="alert">{zh.copyPrivacy}</p><Button disabled={busy} onClick={() => void save()}>{zh.copyIgnoreButton}</Button></>}
    {data.ignored && <p>{zh.copyIgnored}</p>}
    {data.warning && <p role="alert">{data.warning}</p>}{data.error && <p role="alert">{zh.copyFailed}: {data.error}</p>}
  </>}</Card>;
}
