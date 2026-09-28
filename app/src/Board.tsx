import { useEffect, useState } from 'react';
import { Badge, Card, Spinner, Switch } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { zh } from './i18n/zh-CN';
import type { Detail } from './types';
interface Props { project: string; refresh: number; busy: boolean; run: (op: () => Promise<unknown>) => Promise<boolean>; onError: (e: string) => void }
export function Board({ project, refresh, busy, run, onError }: Props) {
  const [detail, setDetail] = useState<Detail | null>(null);
  useEffect(() => {
    let active = true;
    invoke<Detail>('project_detail', { project }).then(d => { if (active) setDetail(d); }).catch(e => { if (active) onError(String(e)); });
    return () => { active = false; };
  }, [project, refresh, onError]);
  if (!detail) return <Spinner label={zh.loading}/>;
  return <div className="board-content">
    <Card className="charter-card">
      <div className="card-heading"><h2>{zh.charter}</h2>{detail.charter && <Badge appearance="tint">v{detail.charter.version}</Badge>}</div>
      {detail.charter ? <><div className="charter-meta"><span>{zh.template}</span><strong>{detail.charter.template}</strong></div><p className="goal">{detail.charter.goal}</p><details><summary>{zh.viewCharter}</summary><pre>{detail.charter.summary}</pre></details></> : <><p>{zh.pendingHint}</p><code>{`bridge-mcp init "${project}"`}</code></>}
    </Card>
    <section><h2 className="section-heading">{zh.roles}</h2><div className="agent-grid">
      {!detail.agents.length && <p className="muted">{zh.noAgents}</p>}
      {detail.agents.map(a => <Card key={a.agent} className="agent-card"><span className={`avatar ${a.agent}`}>{a.agent.slice(0, 1).toUpperCase()}</span><div><strong>{a.agent}</strong><small>{a.role || zh.unassigned}</small></div><Switch disabled={busy} checked={a.enabled} aria-label={`${zh.agentSwitch} ${a.agent}`} onChange={(_, d) => void run(() => invoke('set_switch', { project, agent: a.agent, enabled: d.checked }))}/></Card>)}
    </div></section>
    <section><div className="section-heading count-heading"><h2>{zh.sessions}</h2><Badge appearance="tint">{detail.sessions.length}</Badge></div>
      {!detail.sessions.length && <p className="muted">{zh.noSessions}</p>}
      <div className="session-grid">{detail.sessions.map((s, i) => <Card key={`${s.agent}-${s.session_no}-${i}`} className="session-card">
        <div className="card-heading"><strong>{s.agent} <span className="muted">#{s.session_no}</span></strong><Badge appearance="outline">{s.branch}</Badge></div>
        <p className="session-task">{s.task || zh.noTask}</p>
        <dl>{[[zh.progress, s.progress], [zh.blockers, s.blockers], [zh.nextStep, s.next_step]].filter(([, value]) => value).map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</dl>
        <div className="session-footer"><span title={s.worktree}>{s.worktree}</span><time>{s.updated_at}</time></div>
      </Card>)}</div>
    </section>
    <section><div className="section-heading count-heading"><h2>{zh.claims}</h2><Badge appearance="tint">{detail.claims.length}</Badge></div>
      <Card>{!detail.claims.length ? <p className="muted">{zh.noClaims}</p> : detail.claims.map(c => <div className="claim-row" key={c.path}><code>{c.path}</code><span>{c.agent} #{c.session_no}</span><small title={c.note}>{c.expires_at}</small></div>)}</Card>
    </section>
  </div>;
}
