import { displayPath } from './projectMenuLogic';
import { useEffect, useState } from 'react';
import { Badge, Button, Card, Spinner, Switch } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { zh } from './i18n/zh-CN';
import { RoleEditor } from './RoleEditor';
import type { Management, Detail, Session } from './types';
interface Props { init: () => void; done: () => Promise<void>; project: string; refresh: number; busy: boolean; run: (op: () => Promise<unknown>) => Promise<boolean>; onError: (e: string) => void }
export function Board({ project, refresh, busy, run, onError, init, done }: Props) {
  const [detail, setDetail] = useState<Detail | null>(null);
  const [management, setManagement] = useState<Management | null>(null);
  useEffect(() => {
    let active = true;
    Promise.all([invoke<Detail>('project_detail', { project }), invoke<Management>('management', { project })]).then(([d, m]) => { if (active) { setDetail(d); setManagement(m); } }).catch(e => { if (active) onError(String(e)); });
    return () => { active = false; };
  }, [project, refresh, onError]);
  if (!detail) return <Spinner label={zh.loading}/>;
  return <div className="board-content">
    <Card className="charter-card">
      <div className="card-heading"><h2>{zh.charter}</h2>{detail.charter && <Badge appearance="tint">v{detail.charter.version}</Badge>}</div>
      {detail.charter ? <><div className="charter-meta"><span>{zh.template}</span><strong>{detail.charter.template}</strong></div><p className="goal">{detail.charter.goal}</p><details><summary>{zh.viewCharter}</summary><pre>{detail.charter.summary}</pre></details></> : <><p>{zh.pendingHint}</p><Button appearance="primary" onClick={init}>{zh.startInit}</Button></>}
    </Card>
    <section><h2 className="section-heading">{zh.roles}</h2><div className="agent-grid">
      {!detail.agents.length && <p className="muted">{zh.noAgents}</p>}
      {detail.agents.map(a => <Card key={a.agent} className="agent-card"><span className={`avatar ${a.agent}`}>{a.agent.slice(0, 1).toUpperCase()}</span><div><strong>{a.agent}</strong><small>{a.role || zh.unassigned}</small>{a.role && management && <RoleEditor project={project} agent={a.agent} slot={a.role} roles={management.roles} disabled={busy} done={done}/>}</div><Switch disabled={busy} checked={a.enabled} aria-label={`${zh.agentSwitch} ${a.agent}`} onChange={(_, d) => void run(() => invoke('set_switch', { project, agent: a.agent, enabled: d.checked }))}/></Card>)}
    </div></section>
    <section><div className="section-heading count-heading"><h2>{zh.sessions}</h2><Badge appearance="tint">{detail.sessions.length}</Badge></div>
      {!detail.sessions.length && <p className="muted">{zh.noSessions}</p>}
      <SessionCards sessions={detail.sessions.filter(s => !s.older)}/>
      {!!detail.sessions.filter(s => s.older).length && <details><summary>{zh.olderSessions} ({detail.sessions.filter(s => s.older).length})</summary><SessionCards sessions={detail.sessions.filter(s => s.older)}/></details>}
    </section>
    <section><div className="section-heading count-heading"><h2>{zh.claims}</h2><Badge appearance="tint">{detail.claims.length}</Badge></div>
      <Card>{!detail.claims.length ? <p className="muted">{zh.noClaims}</p> : detail.claims.map(c => <div className="claim-row" key={c.path}><code>{c.path}</code><span>{c.agent} #{c.session_no}</span><small title={c.note}>{c.expires_at}</small></div>)}</Card>
    </section>
  </div>;
}

function SessionCards({ sessions }: { sessions: Session[] }) {
  return <div className="session-grid">{sessions.map(s => <Card key={`${s.agent}-${s.session_no}`} className="session-card">
    <div className="card-heading"><strong>{s.agent} <span className="muted">#{s.session_no}</span></strong><Badge appearance="outline">{s.branch}</Badge></div>
    <p className="session-task">{s.task || zh.noTask}</p>
    <dl>{[[zh.progress, s.progress], [zh.blockers, s.blockers], [zh.nextStep, s.next_step]].filter(([, value]) => value).map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</dl>
    <div className="session-footer"><span title={displayPath(s.worktree)}>{displayPath(s.worktree)}</span><time>{s.updated_at}</time></div>
  </Card>)}</div>;
}
