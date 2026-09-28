import { useEffect, useState } from 'react';
import { Button, Field, Select, Textarea } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { Modal } from './Modal';
import { zh } from './i18n/zh-CN';
import { lines, permissionError } from './wizardLogic';
import type { Role } from './types';

export function RoleEditor({ project, agent, slot, roles, done, disabled }: { project: string; agent: string; slot: string; roles: Role[]; done: () => Promise<void>; disabled: boolean }) {
  const [pending, setPending] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const role = roles.find(r => r.slot === slot);
  async function change() {
    setBusy(true); setError('');
    try { await invoke('set_role', { project, agent, slot: pending }); await done(); setPending(null); }
    catch (e) { setError(String(e)); } finally { setBusy(false); }
  }
  return <div className="role-controls">
    <Select aria-label={`${zh.assignRoles} ${agent}`} disabled={disabled || !role} value={slot} onChange={(_, d) => { setError(''); setPending(d.value); }}>{roles.map(r => <option key={r.slot}>{r.slot}</option>)}</Select>
    <Button size="small" disabled={disabled || !role} onClick={() => setEditing(true)}>{zh.editPermissions}</Button>
    {pending && <Modal title={zh.confirmRole} close={() => setPending(null)} busy={busy} error={error} actions={<Button appearance="primary" disabled={busy} onClick={() => void change()}>{zh.confirm}</Button>}><p>{agent}: {slot} → {pending}</p><p>{zh.roleSwapHelp}</p></Modal>}
    {editing && role && <PermissionEditor project={project} role={role} close={() => setEditing(false)} done={done}/>}
  </div>;
}
function PermissionEditor({ project, role, close, done }: { project: string; role: Role; close: () => void; done: () => Promise<void> }) {
  const [allow, setAllow] = useState(role.allow.join('\n'));
  const [deny, setDeny] = useState(role.deny.join('\n'));
  const [write, setWrite] = useState(role.write.join('\n'));
  const [validation, setValidation] = useState('');
  const [checking, setChecking] = useState(true);
  const [error, setError] = useState('');
  const [reset, setReset] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true; setChecking(true);
    invoke('validate_write', { rules: lines(write) }).then(() => { if (active) setValidation(''); }).catch(e => { if (active) setValidation(permissionError(lines(write), String(e))); }).finally(() => { if (active) setChecking(false); });
    return () => { active = false; };
  }, [write]);
  async function save() {
    setBusy(true); setError('');
    try {
      await invoke('set_permission', { project, slot: role.slot, patch: reset ? { reset: true } : { allow: lines(allow), deny: lines(deny), write: lines(write) } });
      await done(); close();
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }
  return <Modal title={`${zh.editPermissions} · ${role.slot}`} close={close} busy={busy} error={error} actions={<>
    {!reset && <Button disabled={busy} onClick={() => setReset(true)}>{zh.resetPermission}</Button>}
    {reset && <Button disabled={busy} onClick={() => setReset(false)}>{zh.back}</Button>}
    <Button appearance="primary" disabled={busy || (!reset && (checking || !!validation))} onClick={() => void save()}>{reset ? zh.confirm : zh.save}</Button>
  </>}>
    {reset ? <p>{zh.permissionResetHelp}</p> : <div className="form-stack"><p>{zh.permissionHelp}</p>{[[zh.allow, allow, setAllow], [zh.deny, deny, setDeny], [zh.write, write, setWrite]].map(([label, value, setter]) => <Field key={String(label)} label={String(label)}><Textarea rows={3} resize="vertical" value={String(value)} onChange={(_, d) => (setter as (s: string) => void)(d.value)}/></Field>)}{validation && <p className="validation">{validation}</p>}</div>}
  </Modal>;
}
