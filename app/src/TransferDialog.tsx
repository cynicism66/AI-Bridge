import { useEffect, useRef, useState } from 'react';
import { Button, Field, Input, Select, MessageBar, MessageBarBody } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { Modal } from './Modal';
import { zh } from './i18n/zh-CN';
import type { TransferPreview } from './types';
export function TransferDialog({ project, handover, agents, close, done }: { project: string; handover: boolean; agents: string[]; close: () => void; done: () => Promise<void> }) {
  const [out, setOut] = useState<string>(zh.exportDefault);
  const [to, setTo] = useState(agents[0] || '');
  const [draft, setDraft] = useState<TransferPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const id = useRef<number | null>(null);
  useEffect(() => () => { if (id.current !== null) void invoke('cancel_preview', { id: id.current }).catch(() => {}); }, []);
  async function preview() {
    setBusy(true); setError('');
    try { const d = await invoke<TransferPreview>('preview_transfer', { project, out: handover ? null : out, to: handover ? to : null }); id.current = d.id; setDraft(d); }
    catch (e) { setError(String(e)); } finally { setBusy(false); }
  }
  async function confirm() {
    if (!draft) return;
    setBusy(true); setError('');
    try { await invoke('confirm_preview', { id: draft.id }); id.current = null; await done(); close(); }
    catch (e) { setError(String(e)); setDraft(null); } finally { setBusy(false); }
  }
  return <Modal title={handover ? zh.handover : zh.exportRecords} close={close} busy={busy} error={error} actions={<>
    {draft && <Button disabled={busy} onClick={() => { void invoke('cancel_preview', { id: draft.id }); id.current = null; setDraft(null); }}>{zh.back}</Button>}
    <Button appearance="primary" disabled={busy || (!draft && (handover ? !to : !out.trim()))} onClick={() => void (draft ? confirm() : preview())}>{draft ? zh.writeConfirmed : zh.preview}</Button>
  </>}>
    {!draft ? <div className="form-stack">{handover ? <><p>{zh.handoverHelp}</p><Field label={zh.handoverTo}><Select value={to} onChange={(_, d) => setTo(d.value)}>{agents.map(a => <option key={a}>{a}</option>)}</Select></Field></> : <><Field label={zh.outputPath}><Input value={out} onChange={(_, d) => setOut(d.value)}/></Field><Button onClick={() => { void save({ defaultPath: `${project}/${zh.exportDefault}`, filters: [{ name: 'Markdown', extensions: ['md'] }] }).then(p => { if (p) setOut(p); }).catch(e => setError(String(e))); }}>{zh.chooseOutput}</Button></>}</div> : <div className="form-stack">
      {!!draft.preview.remotes.length && <MessageBar intent="warning"><MessageBarBody>{zh.remoteWarning}<ul>{draft.preview.remotes.map(r => <li key={r}>{r}</li>)}</ul></MessageBarBody></MessageBar>}
      <strong>{draft.preview.summary}</strong><div><strong>{zh.outputFiles}</strong>{draft.preview.paths.map(p => <p key={p}>{p}</p>)}</div><pre className="preview-body">{draft.preview.body}</pre>
    </div>}
  </Modal>;
}
