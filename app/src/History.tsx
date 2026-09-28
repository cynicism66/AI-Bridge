import { useEffect, useRef, useState } from 'react';
import { Button, Card, Checkbox, Field, Input, Spinner } from '@fluentui/react-components';
import { History20Regular, Chat20Regular, LockClosed20Regular, People20Regular } from '@fluentui/react-icons';
import { invoke } from '@tauri-apps/api/core';
import { zh } from './i18n/zh-CN';
import { filterError } from './wizardLogic';
import type { HistoryFilter, HistoryPage, Management } from './types';
const empty: HistoryFilter = { agents: [], kinds: [], from: '', until: '', keyword: '' };
const kinds = { status: zh.eventStatus, message: zh.eventMessage, claim: zh.eventClaim, release: zh.eventRelease, expire: zh.eventExpire, switch: zh.eventSwitch, agent_switch: zh.eventAgentSwitch, init: zh.eventInit, role: zh.eventRole, handover: zh.eventHandover, permission: zh.eventPermission };
const toggle = (values: string[], value: string) => values.includes(value) ? values.filter(x => x !== value) : [...values, value];
export function History({ project, refresh, onError }: { project: string; refresh: number; onError: (e: string) => void }) {
  const [draft, setDraft] = useState<HistoryFilter>(empty);
  const [filter, setFilter] = useState<HistoryFilter>(empty);
  const [agents, setAgents] = useState<string[]>([]);
  const [page, setPage] = useState<HistoryPage>({ events: [], before: null });
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  useEffect(() => {
    generation.current += 1;
    let active = true; setBusy(true);
    Promise.all([invoke<HistoryPage>('history_page', { project, filter }), invoke<Management>('management', { project })]).then(([p, m]) => { if (active) { setPage(p); setAgents([...new Set([...m.known, 'human', 'bridge'])]); } }).catch(e => { if (active) onError(String(e)); }).finally(() => { if (active) setBusy(false); });
    return () => { active = false; generation.current += 1; };
  }, [project, filter, refresh, onError]);
  async function more() {
    const current = generation.current;
    setBusy(true);
    try { const next = await invoke<HistoryPage>('history_page', { project, filter: { ...filter, before: page.before } }); if (current === generation.current) setPage(p => ({ events: [...p.events, ...next.events], before: next.before })); }
    catch (e) { if (current === generation.current) onError(String(e)); } finally { if (current === generation.current) setBusy(false); }
  }
  return <div className="board-content"><Card><h2>{zh.historyFilters}</h2><div className="form-stack">
    <Field label={zh.filterAgents}><div className="check-list">{agents.map(a => <Checkbox key={a} label={a} checked={draft.agents.includes(a)} onChange={() => setDraft(d => ({ ...d, agents: toggle(d.agents, a) }))}/>)}</div></Field>
    <Field label={zh.filterKinds}><div className="check-list">{Object.entries(kinds).map(([k, label]) => <Checkbox key={k} label={label} checked={draft.kinds.includes(k)} onChange={() => setDraft(d => ({ ...d, kinds: toggle(d.kinds, k) }))}/>)}</div></Field>
    <div className="filter-row"><Field label={zh.dateFrom}><Input type="date" value={draft.from} onChange={(_, d) => setDraft(f => ({ ...f, from: d.value }))}/></Field><Field label={zh.dateUntil}><Input type="date" value={draft.until} onChange={(_, d) => setDraft(f => ({ ...f, until: d.value }))}/></Field><Field label={zh.keyword}><Input value={draft.keyword} onChange={(_, d) => setDraft(f => ({ ...f, keyword: d.value }))}/></Field></div>
    {filterError(draft) && <p className="validation">{filterError(draft)}</p>}<div className="button-row"><Button appearance="primary" disabled={busy || !!filterError(draft)} onClick={() => setFilter({ ...draft })}>{zh.applyFilters}</Button><Button disabled={busy} onClick={() => { setDraft(empty); setFilter({ ...empty }); }}>{zh.clearFilters}</Button></div>
  </div></Card>
    {page.events.map(e => <Card key={e.id} className="event-card"><div className="event-description">{e.kind === 'message' ? <Chat20Regular/> : e.kind === 'permission' ? <LockClosed20Regular/> : ['init', 'role', 'handover'].includes(e.kind) ? <People20Regular/> : <History20Regular/>}<span>{e.description}</span></div><details><summary>{zh.details}</summary><pre>{JSON.stringify(e.detail, null, 2)}</pre></details></Card>)}
    {!page.events.length && !busy && <p className="muted">{zh.noEvents}</p>}
    <p className="muted">{page.before ? zh.onlyRecent.replace('{count}', String(page.events.length)) : page.events.length ? zh.allEventsLoaded : ''}</p>
    {page.before && <Button disabled={busy} onClick={() => void more()}>{zh.loadMore}</Button>}{busy && <Spinner size="tiny"/>}
  </div>;
}
