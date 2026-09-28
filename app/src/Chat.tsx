import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { Badge, Button, Select, Spinner, Textarea } from '@fluentui/react-components';
import { Send20Regular } from '@fluentui/react-icons';
import { invoke } from '@tauri-apps/api/core';
import { MessageReceipts } from './MessageReceipts';
import { zh } from './i18n/zh-CN';
import { actor, mergeMessages, source } from './logic';
import type { Message, MessagePage } from './types';
interface Props { project: string; refresh: number; onError: (s: string) => void; reload: () => Promise<void> }
export function Chat({ project, refresh, onError, reload }: Props) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [before, setBefore] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [paging, setPaging] = useState(false);
  const [content, setContent] = useState('');
  const [to, setTo] = useState('all');
  const [sending, setSending] = useState(false);
  const initial = useRef(true);
  const scroller = useRef<HTMLDivElement>(null);
  const lastRead = useRef(0);
  const earliestId = useRef(0);
  const scrollAfterSend = useRef(false);
  useLayoutEffect(() => {
    if (scrollAfterSend.current && scroller.current) {
      scroller.current.scrollTop = scroller.current.scrollHeight;
      scrollAfterSend.current = false;
    }
  }, [messages]);
  useEffect(() => {
    let active = true;
    async function load() {
      try {
        const page = await invoke<MessagePage>('message_page', { project, before: null });
        if (!active) return;
        let incoming = page.messages;
        let cursor = page.before;
        while (earliestId.current && cursor && incoming[0]?.id > earliestId.current) {
          const gap = await invoke<MessagePage>('message_page', { project, before: cursor });
          if (!active) return;
          incoming = mergeMessages(gap.messages, incoming);
          cursor = gap.before;
        }
        if (earliestId.current) incoming = incoming.filter(m => m.id >= earliestId.current);
        const nearEnd = scrollAfterSend.current || !scroller.current || scroller.current.scrollHeight - scroller.current.scrollTop - scroller.current.clientHeight < 120;
        scrollAfterSend.current = nearEnd;
        setMessages(old => mergeMessages(old, incoming));
        earliestId.current = incoming[0]?.id || earliestId.current;
        if (initial.current) { setBefore(page.before); initial.current = false; }
        setLoading(false);
        const through = page.messages.at(-1)?.id || 0;
        if (through > lastRead.current) {
          await invoke('mark_read', { project, through });
          lastRead.current = through;
          if (active) await reload();
        }
      } catch (e) { if (active) { setLoading(false); onError(String(e)); } }
    }
    void load();
    return () => { active = false; };
  }, [project, refresh, onError, reload]);
  async function older() {
    if (!before) return;
    setPaging(true);
    try {
      const height = scroller.current?.scrollHeight || 0;
      const page = await invoke<MessagePage>('message_page', { project, before });
      earliestId.current = page.messages[0]?.id || earliestId.current;
      setMessages(old => mergeMessages(page.messages, old)); setBefore(page.before);
      requestAnimationFrame(() => { if (scroller.current) scroller.current.scrollTop += scroller.current.scrollHeight - height; });
    } catch (e) { onError(String(e)); } finally { setPaging(false); }
  }
  async function send() {
    if (!content.trim() || sending) return;
    setSending(true);
    try { await invoke('send_message', { project, content, to }); scrollAfterSend.current = true; setContent(''); await reload(); }
    catch (e) { onError(String(e)); } finally { setSending(false); }
  }
  return <div className="chat-content">
    <div className="messages" ref={scroller} aria-live="polite">
      {before && <Button className="older" appearance="subtle" disabled={paging} onClick={() => void older()}>{zh.loadOlder}</Button>}
      {loading ? <Spinner label={zh.loading}/> : !messages.length && <p className="empty-chat">{zh.emptyMessages}</p>}
      {messages.map(m => <article className={`message ${m.sender === 'human' ? 'from-human' : ''}`} key={m.id}>
        <span className={`avatar ${m.sender}`}>{actor(m.sender).slice(0, 1).toUpperCase()}</span>
        <div className="message-main"><div className="message-heading"><strong>{actor(m.sender)}</strong><span className="message-id">#{m.id}</span><span>→ {actor(m.recipient)}</span>{m.sender === 'human' && <Badge size="small" appearance="tint">{source(m.via)}</Badge>}<time>{m.created_at}</time></div><p className="message-body">{m.content}</p><MessageReceipts message={m}/></div>
      </article>)}
    </div>
    <form className="composer" onSubmit={e => { e.preventDefault(); void send(); }}>
      <Textarea value={content} onChange={(_, d) => setContent(d.value)} placeholder={zh.messagePlaceholder} aria-label={zh.messagePlaceholder} resize="vertical" disabled={sending}/>
      <div className="composer-actions"><label>{zh.to}<Select value={to} onChange={(_, d) => setTo(d.value)} disabled={sending}><option value="all">{zh.all}</option><option value="claude">claude</option><option value="codex">codex</option></Select></label><Button type="submit" appearance="primary" icon={<Send20Regular/>} disabled={sending || !content.trim()}>{sending ? zh.sending : zh.send}</Button></div>
    </form>
  </div>;
}
