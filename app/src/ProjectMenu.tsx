import { useState, type ReactElement } from 'react';
import { Menu, MenuTrigger, MenuPopover, MenuList, MenuItem } from '@fluentui/react-components';
import { invoke } from '@tauri-apps/api/core';
import { zh } from './i18n/zh-CN';
import { projectActions, type ProjectAction } from './projectMenuLogic';
import { PurgeDialog } from './PurgeDialog';
interface Props { project: string; discovered?: boolean; enabled: boolean; busy: boolean; toggle: (project: string, enabled: boolean) => Promise<boolean>; run: (op: () => Promise<unknown>) => Promise<boolean>; children: ReactElement }
export function ProjectMenu({ project, discovered = false, enabled, busy, toggle, run, children }: Props) {
  const [purging, setPurging] = useState(false);
  const labels: Record<ProjectAction, string> = { enable: zh.enableCollaboration, disable: zh.disableCollaboration, open: zh.openFolder, forget: zh.forgetProject, purge: zh.purgeProject, hide: zh.hideProject };
  function action(a: ProjectAction) {
    if (a === 'purge') { setPurging(true); return; }
    if (a === 'enable' || a === 'disable') { void toggle(project, a === 'enable'); return; }
    void run(() => invoke({ open: 'open_project_folder', forget: 'forget_project', hide: 'hide_project' }[a], { project }));
  }
  return <><Menu openOnContext positioning="below-start"><MenuTrigger disableButtonEnhancement>{children}</MenuTrigger><MenuPopover><MenuList>{projectActions(discovered, enabled).map(a => a === 'purge' ? <MenuItem key={a} className="danger-menu" disabled={busy} onClick={() => action(a)}>{labels[a]}</MenuItem> : <MenuItem key={a} disabled={busy} onClick={() => action(a)}>{labels[a]}</MenuItem>)}</MenuList></MenuPopover></Menu>{purging && <PurgeDialog project={project} close={() => setPurging(false)} run={run}/>}</>;
}
