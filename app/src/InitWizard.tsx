import { displayPath } from './projectMenuLogic';
import { useEffect, useRef, useState } from 'react';
import { Button, Checkbox, Field, Input, Select, Textarea, Spinner } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Modal } from './Modal';
import { zh } from './i18n/zh-CN';
import { validAgent, wizardError } from './wizardLogic';
import type { Management, TemplateInfo } from './types';

export function InitWizard({ project, close, done }: { project: string; close: () => void; done: () => Promise<void> }) {
  const [step, setStep] = useState(0);
  const [known, setKnown] = useState<string[]>(['claude', 'codex']);
  const [participants, setParticipants] = useState<string[]>(['claude', 'codex']);
  const [newAgent, setNewAgent] = useState('');
  const [name, setName] = useState<string>(zh.taskTemplate);
  const [file, setFile] = useState<string | null>(null);
  const [template, setTemplate] = useState<TemplateInfo | null>(null);
  const [assignments, setAssignments] = useState<Record<string,string>>({});
  const [goal, setGoal] = useState('');
  const [writeRules, setWriteRules] = useState(true);
  const [kickoff, setKickoff] = useState(true);
  const [preview, setPreview] = useState<{ id: number; charter: string } | null>(null);
  const previewId = useRef<number | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    invoke<Management>('management', { project }).then(m => { if (active) { setKnown(m.known); setGoal(m.charter?.goal || ''); } }).catch(e => { if (active) setError(String(e)); });
    return () => { active = false; if (previewId.current !== null) void invoke('cancel_preview', { id: previewId.current }).catch(() => {}); };
  }, [project]);
  useEffect(() => {
    let active = true;
    setTemplate(null); setAssignments({});
    if (name !== zh.customTemplate || file) {
      invoke<TemplateInfo>('template_info', { name, file: name === zh.customTemplate ? file : null }).then(t => {
        if (active) { setTemplate(t); setError(''); }
      }).catch(e => { if (active) setError(String(e)); });
    }
    return () => { active = false; };
  }, [name, file]);
  const validation = wizardError(step, participants, template, assignments, goal);
  async function next() {
    if (step < 4) { setError(''); setStep(step + 1); return; }
    setBusy(true); setError('');
    try {
      const p = await invoke<{ id: number; charter: string }>('preview_init', { project, request: { template: name, template_file: name === zh.customTemplate ? file : null, roles: template!.roles.map(r => `${r.slot}=${assignments[r.slot]}`), goal, write_rules: writeRules, no_kickoff: !kickoff } });
      previewId.current = p.id; setPreview(p); setStep(5);
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }
  async function confirm() {
    if (!preview) return;
    setBusy(true); setError('');
    try { await invoke('confirm_preview', { id: preview.id }); previewId.current = null; await done(); close(); }
    catch (e) { setError(String(e)); setPreview(null); } finally { setBusy(false); }
  }
  const steps = [zh.participants, zh.template, zh.assignRoles, zh.goal, zh.initOptions, zh.preview];
  return <Modal title={zh.startInit} close={close} busy={busy} error={error} actions={<>
    {step > 0 && <Button disabled={busy} onClick={() => { if (previewId.current !== null) void invoke('cancel_preview', { id: previewId.current }); previewId.current = null; setPreview(null); setStep(step - 1); setError(''); }}>{zh.back}</Button>}
    {step < 5 ? <Button appearance="primary" disabled={busy || !!validation} onClick={() => void next()}>{step === 4 ? zh.preview : zh.next}</Button> : <Button appearance="primary" disabled={busy || !preview} onClick={() => void confirm()}>{zh.confirm}</Button>}
  </>}>
    <p className="muted">{displayPath(project)}</p><div className="wizard-steps">{steps.map((label, i) => <span key={label} className={i === step ? 'current' : ''}>{i + 1}. {label}</span>)}</div>
    <div className="form-stack">
      {step === 0 && <><div className="check-list">{known.map(a => <Checkbox key={a} label={a} checked={participants.includes(a)} onChange={(_, d) => setParticipants(p => d.checked ? [...p, a] : p.filter(x => x !== a))}/>)}</div><Field label={zh.agentName}><Input value={newAgent} placeholder={zh.newAgentHint} onChange={(_, d) => setNewAgent(d.value)}/></Field><Button disabled={!validAgent(newAgent)} onClick={() => { const a = newAgent.trim().toLowerCase(); setKnown(k => [...new Set([...k, a])]); setParticipants(p => [...new Set([...p, a])]); setNewAgent(''); }}>{zh.addAgent}</Button></>}
      {step === 1 && <><Field label={zh.template}><Select value={name} onChange={(_, d) => setName(d.value)}>{[zh.taskTemplate, zh.pairTemplate, zh.soloTemplate, zh.customTemplate].map(n => <option key={n}>{n}</option>)}</Select></Field><p>{({ [zh.taskTemplate]: zh.taskDescription, [zh.pairTemplate]: zh.pairDescription, [zh.soloTemplate]: zh.soloDescription, [zh.customTemplate]: zh.customDescription })[name]}</p>{name === zh.customTemplate && <><Button onClick={() => { void open({ multiple: false, filters: [{ name: 'TOML', extensions: ['toml'] }] }).then(p => { if (typeof p === 'string') setFile(p); }).catch(e => setError(String(e))); }}>{zh.chooseTemplate}</Button><small>{file && displayPath(file)}</small></>}{template && <ul>{template.roles.map(r => <li key={r.slot}><strong>{r.slot}</strong> · {r.duties}</li>)}</ul>}</>}
      {step === 2 && template?.roles.map(r => <Field key={r.slot} label={r.slot}><Select value={assignments[r.slot] || ''} onChange={(_, d) => setAssignments(v => ({ ...v, [r.slot]: d.value }))}><option value="">—</option>{participants.map(a => <option key={a}>{a}</option>)}</Select></Field>)}
      {step === 3 && <Field label={zh.goal}><Textarea resize="vertical" rows={5} value={goal} onChange={(_, d) => setGoal(d.value)}/></Field>}
      {step === 4 && <><Checkbox label={zh.writeRules} checked={writeRules} onChange={(_, d) => setWriteRules(d.checked === true)}/><Checkbox label={zh.kickoff} checked={kickoff} onChange={(_, d) => setKickoff(d.checked === true)}/><p className="muted">{zh.initWarning}</p></>}
      {step === 5 && <><p>{zh.fullPreview}</p><pre className="preview-body">{preview?.charter}</pre></>}
      {validation && <p className="validation">{validation}</p>}{busy && <Spinner size="tiny"/>}
    </div>
  </Modal>;
}
