import { displayPath } from './projectMenuLogic';
import { useEffect, useState } from 'react';
import { Badge, Button, FluentProvider, MessageBar, MessageBarActions, MessageBarBody, Spinner, Tab as FluentTab, TabList, webDarkTheme, webLightTheme } from '@fluentui/react-components';
import { Dismiss20Regular, Board20Regular, Chat20Regular, History20Regular, Settings20Regular } from '@fluentui/react-icons';
import { zh } from './i18n/zh-CN';
import { Sidebar } from './Sidebar';
import { Board } from './Board';
import { InitWizard } from './InitWizard';
import { History } from './History';
import { Settings } from './Settings';
import { Chat } from './Chat';
import { projectName } from './logic';
import { useWorkspace } from './useWorkspace';
import type { Tab } from './types';
export function App() {
  const w = useWorkspace();
  const [wizard, setWizard] = useState<string | null>(null);
  const [dark, setDark] = useState(matchMedia('(prefers-color-scheme: dark)').matches);
  useEffect(() => {
    const media = matchMedia('(prefers-color-scheme: dark)');
    const changed = () => setDark(media.matches);
    media.addEventListener('change', changed);
    return () => media.removeEventListener('change', changed);
  }, []);
  const project = w.data?.projects.find(p => p.project === w.project);
  return <FluentProvider theme={dark ? webDarkTheme : webLightTheme} className={`shell ${dark ? 'dark' : ''}`}>
    {w.data && <Sidebar data={w.data} selected={w.project} busy={w.busy} select={w.select} run={w.run} init={setWizard} settings={() => w.setTab('settings')}/>}
    <main>
      {w.error && <MessageBar intent="error"><MessageBarBody>{w.error}</MessageBarBody><MessageBarActions containerAction={<Button appearance="transparent" icon={<Dismiss20Regular/>} aria-label={zh.dismiss} onClick={() => w.setError('')}/>}><Button onClick={() => void w.run(w.reload)}>{zh.retry}</Button></MessageBarActions></MessageBar>}
      {!w.data ? <div className="center"><Spinner label={zh.loading}/></div> : !w.project ? w.tab === 'settings' ? <Settings project={null} refresh={w.refresh} init={() => {}} done={w.reload} onError={w.setError}/> : <div className="center"><Board20Regular/><h2>{zh.emptySelection}</h2></div> : <>
        <header className="project-header"><div><div className="eyebrow">{zh.appName} / {zh.projects}</div><h1>{projectName(w.project)}<Badge color={project?.enabled ? 'success' : 'subtle'} appearance="tint">{project?.enabled ? zh.enabled : zh.disabled}</Badge>{!project?.initialized && <Badge color="warning" appearance="tint">{zh.pending}</Badge>}</h1><p title={displayPath(w.project)}>{displayPath(w.project)}</p></div></header>
        <TabList selectedValue={w.tab} onTabSelect={(_, d) => w.setTab(d.value as Tab)} className="tabs">
          <FluentTab value="board" icon={<Board20Regular/>}>{zh.board}</FluentTab><FluentTab value="chat" icon={<Chat20Regular/>}>{zh.chat}{!!project?.unread && <Badge size="small" color="danger">{project.unread}</Badge>}</FluentTab><FluentTab value="history" icon={<History20Regular/>}>{zh.history}</FluentTab><FluentTab value="settings" icon={<Settings20Regular/>}>{zh.settings}</FluentTab>
        </TabList>
        <div className={`tab-content ${w.tab === 'chat' ? 'chat-tab' : ''}`}>
          {w.tab === 'board' && <Board key={w.project} project={w.project} refresh={w.refresh} busy={w.busy} run={w.run} onError={w.setError} init={() => setWizard(w.project)} done={w.reload}/>}
          {w.tab === 'chat' && <Chat key={w.project} project={w.project} refresh={w.refresh} reload={w.reload} onError={w.setError}/>}
          {w.tab === 'history' && <History key={w.project} project={w.project} refresh={w.refresh} onError={w.setError}/>}
          {w.tab === 'settings' && <Settings key={w.project} project={w.project} refresh={w.refresh} init={() => setWizard(w.project)} done={w.reload} onError={w.setError}/>}
        </div>
      </>}
    </main>
    {wizard && <InitWizard key={wizard} project={wizard} close={() => setWizard(null)} done={w.reload}/>}
  </FluentProvider>;
}
