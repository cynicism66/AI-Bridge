import { Badge, Button, Switch, Tooltip } from '@fluentui/react-components';
import { Add20Regular, Folder20Regular, EyeOff20Regular } from '@fluentui/react-icons';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { zh } from './i18n/zh-CN';
import { projectName, trayState, unreadTotal } from './logic';
import type { Snapshot } from './types';
interface Props { data: Snapshot; selected: string | null; busy: boolean; select: (p: string) => void; run: (op: () => Promise<unknown>) => Promise<boolean> }
export function Sidebar({ data, selected, busy, select, run }: Props) {
  const toggle = (project: string, enabled: boolean) => run(() => invoke('set_switch', { project, agent: null, enabled }));
  async function add() {
    const folder = await open({ directory: true, multiple: false, title: zh.chooseFolder });
    if (typeof folder === 'string') { await invoke('set_switch', { project: folder, agent: null, enabled: true }); }
  }
  return <aside className="sidebar">
    <div className="brand"><span className="brand-mark" aria-hidden="true">B</span><div><strong>{zh.appName}</strong><small>{zh.tagline}</small></div></div>
    <div className="project-scroll">
      <div className="section-label">{zh.projects}<Badge appearance="tint">{data.projects.length}</Badge></div>
      {!data.projects.length && <p className="muted empty-list">{zh.emptyProjects}</p>}
      {data.projects.map(p => <div className={`project-row ${selected === p.project ? 'selected' : ''}`} key={p.project}>
        <button className="project-select" onClick={() => select(p.project)} title={p.project}>
          <Folder20Regular/><span><strong>{projectName(p.project)}</strong><small>{p.initialized ? p.project : zh.pending}</small></span>
          {p.unread > 0 && <Badge color="danger" size="small" aria-label={zh.unread}>{p.unread}</Badge>}
        </button>
        <Switch checked={p.enabled} disabled={busy} aria-label={`${zh.projectSwitch} ${p.project}`} onChange={(_, d) => void toggle(p.project, d.checked)}/>
      </div>)}
      <div className="section-label discovered-label">{zh.discovered}</div>
      {!data.discovered.length && <p className="muted empty-list">{zh.noDiscovery}</p>}
      {data.discovered.map(p => <div className="discovered-row" key={p.project}>
        <button className="project-select" title={p.project} onClick={() => select(p.project)}><Folder20Regular/><span><strong>{projectName(p.project)}</strong><small>{p.project}</small></span></button>
        <Tooltip content={zh.hideProject} relationship="label"><Button size="small" appearance="subtle" icon={<EyeOff20Regular/>} disabled={busy} onClick={() => void run(() => invoke('hide_project', { project: p.project }))}/></Tooltip>
        <Switch checked={false} disabled={busy} aria-label={`${zh.projectSwitch} ${p.project}`} onChange={() => void toggle(p.project, true)}/>
      </div>)}
      <Button className="add-project" appearance="subtle" icon={<Add20Regular/>} disabled={busy} onClick={() => void run(add)}>{zh.addProject}</Button>
    </div>
    <div className="global-control"><div><strong>{zh.globalSwitch}</strong><small>{data.global ? zh.globalOn : zh.globalOff}</small></div><Switch checked={data.global} disabled={busy} aria-label={zh.globalSwitch} onChange={(_, d) => void run(() => invoke('set_switch', { project: null, agent: null, enabled: d.checked }))}/></div>
    <div className="sidebar-foot"><span className={`status-dot ${trayState(data.global, unreadTotal(data.projects))}`}/>{data.global ? zh.enabled : zh.disabled}<span>{zh.unread} · {unreadTotal(data.projects)}</span></div>
  </aside>;
}
