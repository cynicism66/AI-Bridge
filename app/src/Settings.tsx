import { displayPath } from './projectMenuLogic';
import { useEffect, useState } from 'react';
import { Button, Card, Switch, Spinner } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { zh } from './i18n/zh-CN';
import { CopySettings } from './CopySettings';
import { TransferDialog } from './TransferDialog';
import type { Management, Settings as SettingsData } from './types';
export function Settings({ project, refresh, init, done, onError }: { project: string | null; refresh: number; init: () => void; done: () => Promise<void>; onError: (e: string) => void }) {
  const [data, setData] = useState<SettingsData | null>(null);
  const [management, setManagement] = useState<Management | null>(null);
  const [busy, setBusy] = useState(false);
  const [transfer, setTransfer] = useState<'export' | 'handover' | null>(null);
  useEffect(() => {
    let active = true;
    invoke<SettingsData>('app_settings').then(d => { if (active) setData(d); }).catch(e => { if (active) onError(String(e)); });
    if (project) invoke<Management>('management', { project }).then(d => { if (active) setManagement(d); }).catch(e => { if (active) onError(String(e)); });
    return () => { active = false; };
  }, [project, refresh, onError]);
  async function save(patch: unknown) {
    setBusy(true);
    try { await invoke('save_settings', { patch }); await done(); }
    catch (e) { onError(String(e)); } finally { setBusy(false); }
  }
  if (!data) return <Spinner label={zh.loading}/>;
  return <div className="board-content">
    {project && <Card><h2>{zh.projectSettings}</h2><div className="button-row"><Button onClick={init}>{management?.charter ? zh.reinit : zh.startInit}</Button><Button onClick={() => setTransfer('export')}>{zh.exportRecords}</Button><Button disabled={!management?.charter} onClick={() => setTransfer('handover')}>{zh.handover}</Button></div>{management?.charter && <><p>{zh.template}: {management.charter.template} · v{management.charter.version}</p><details><summary>{zh.viewCharter}</summary><pre>{management.charter.summary}</pre></details></>}</Card>}
    {project && <CopySettings key={project} project={project} refresh={refresh} done={done} onError={onError}/>}
    <Card><h2>{zh.softwareSettings}</h2><Switch checked={data.preferences.notifications_enabled} disabled={busy} label={zh.notificationsEnabled} onChange={(_, d) => void save({ notifications_enabled: d.checked })}/><Button disabled={busy} onClick={() => void save({ reset_close_tip: true })}>{zh.resetCloseTip}</Button><h3>{zh.hiddenProjects}</h3>{!data.preferences.hidden_projects.length && <p className="muted">{zh.noHidden}</p>}{data.preferences.hidden_projects.map(p => <div className="hidden-row" key={p}><span>{displayPath(p)}</span><Button disabled={busy} onClick={() => void save({ unhide: p })}>{zh.unhide}</Button></div>)}<dl><dt>{zh.databasePath}</dt><dd className="path-value">{displayPath(data.database)}</dd><dt>{zh.softwareVersion}</dt><dd>{data.version}</dd></dl></Card>
    {transfer && project && <TransferDialog project={project} agents={management?.known || []} handover={transfer === 'handover'} close={() => setTransfer(null)} done={done}/>}
  </div>;
}
