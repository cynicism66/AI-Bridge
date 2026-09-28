import { useEffect, useState } from 'react';
import { Button, Checkbox, MessageBar, MessageBarBody } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { Modal } from './Modal';
import { zh } from './i18n/zh-CN';
import { displayPath } from './projectMenuLogic';
interface Info { active_sessions: number; write_rules: boolean }
export function PurgeDialog({ project, close, run }: { project: string; close: () => void; run: (op: () => Promise<unknown>) => Promise<boolean> }) {
  const [info, setInfo] = useState<Info | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [force, setForce] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => { let active = true; invoke<Info>('purge_info', { project }).then(i => { if (active) setInfo(i); }).catch(e => { if (active) setError(String(e)); }); return () => { active = false; }; }, [project]);
  async function remove() {
    setBusy(true); setError('');
    const ok = await run(async () => {
      try { await invoke('purge_project', { project, force }); }
      catch (e) { setError(String(e)); setForce(false); setInfo(await invoke<Info>('purge_info', { project })); throw e; }
    });
    setBusy(false); if (ok) close();
  }
  return <Modal title={confirm ? zh.purgeFinalTitle : zh.purgeProject} close={close} busy={busy} error={error} actions={<>
    {confirm && <Button disabled={busy} onClick={() => { setConfirm(false); setForce(false); }}>{zh.back}</Button>}
    <Button appearance="primary" className={confirm ? 'danger-button' : ''} disabled={busy || !info || (confirm && !!info.active_sessions && !force)} onClick={() => confirm ? void remove() : setConfirm(true)}>{confirm ? zh.purgeConfirmed : zh.continuePurge}</Button>
  </>}><p className="path-value">{displayPath(project)}</p><MessageBar intent="warning"><MessageBarBody>{zh.purgeWarning}</MessageBarBody></MessageBar><p>{zh.purgeNoFiles}</p><p>{info?.write_rules ? zh.purgeRulesRemain : zh.purgeRulesConditional}</p>{confirm && <p>{zh.purgeFinalWarning}</p>}{confirm && !!info?.active_sessions && <><MessageBar intent="warning"><MessageBarBody>{zh.purgeActive}</MessageBarBody></MessageBar><Checkbox checked={force} label={zh.purgeForce} onChange={(_, d) => setForce(d.checked === true)}/></>}</Modal>;
}
