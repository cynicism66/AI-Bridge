import { useLayoutEffect, useRef, useState } from 'react';
import { Button, Select, Textarea } from '@fluentui/react-components';
import { Send20Regular } from '@fluentui/react-icons';
import { zh } from './i18n/zh-CN';

interface Props { send: (content: string, to: string) => Promise<void>; onError: (error: string) => void }

export function resizeComposer(input: HTMLTextAreaElement) {
  const css = getComputedStyle(input);
  const line = Number.parseFloat(css.lineHeight) || 20;
  const padding = (Number.parseFloat(css.paddingTop) || 0) + (Number.parseFloat(css.paddingBottom) || 0);
  const border = (Number.parseFloat(css.borderTopWidth) || 0) + (Number.parseFloat(css.borderBottomWidth) || 0);
  input.style.height = '0px';
  const height = Math.max(line * 3 + padding, Math.min(input.scrollHeight, line * 8 + padding));
  input.style.height = `${height + border}px`;
  input.style.overflowY = input.scrollHeight > height ? 'auto' : 'hidden';
}

export function Composer({ send, onError }: Props) {
  const [content, setContent] = useState('');
  const [to, setTo] = useState('all');
  const [sending, setSending] = useState(false);
  const busy = useRef(false);
  const input = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => { if (input.current) resizeComposer(input.current); }, [content]);
  useLayoutEffect(() => {
    const element = input.current;
    if (!element || typeof ResizeObserver === 'undefined') return;
    let width = element.clientWidth;
    const observer = new ResizeObserver(() => {
      if (width !== element.clientWidth) { width = element.clientWidth; resizeComposer(element); }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  async function submit() {
    if (!content.trim() || busy.current) return;
    busy.current = true; setSending(true);
    try { await send(content, to); setContent(''); }
    catch (error) { onError(String(error)); }
    finally { busy.current = false; setSending(false); }
  }
  return <form className="composer" onSubmit={e => { e.preventDefault(); void submit(); }}>
    <Textarea textarea={{ ref: input, rows: 3 }} value={content} onChange={(_, d) => setContent(d.value)}
      placeholder={zh.messagePlaceholder} aria-label={zh.messagePlaceholder} resize="none" disabled={sending}
      onKeyDown={e => {
        if (e.key !== 'Enter' || e.shiftKey || e.nativeEvent.isComposing || e.nativeEvent.keyCode === 229) return;
        e.preventDefault(); void submit();
      }}/>
    <div className="composer-actions"><label>{zh.to}<Select value={to} onChange={(_, d) => setTo(d.value)} disabled={sending}>
      <option value="all">{zh.all}</option><option value="claude">claude</option><option value="codex">codex</option>
    </Select></label><Button type="submit" appearance="primary" icon={<Send20Regular/>} disabled={sending || !content.trim()}>{sending ? zh.sending : zh.send}</Button></div>
  </form>;
}
