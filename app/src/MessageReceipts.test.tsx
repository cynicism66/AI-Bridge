import { expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { MessageReceipts } from './MessageReceipts';
import { zh } from './i18n/zh-CN';
import type { Message } from './types';

const message: Message = { id: 141, project: '/test', sender: 'human', recipient: 'all', content: 'test', created_at: '', via: 'gui', receipts: [
  { agent: 'claude', read: true, read_at: '2090-01-02 16:10:20' },
  { agent: 'codex', read: false, read_at: null },
] };
it('renders each recipient and first read time for human messages', () => {
  const html = renderToStaticMarkup(<MessageReceipts message={message}/>);
  expect(html).toContain(`claude ${zh.messageRead}`);
  expect(html).toContain('16:10</time>');
  expect(html).toContain(` · codex ${zh.messageUnread}`);
});
it('preserves historical read status without inventing a timestamp', () => {
  const html = renderToStaticMarkup(<MessageReceipts message={{ ...message, receipts: [{ agent: 'claude', read: true, read_at: null }] }}/>);
  expect(html).toContain(zh.messageRead);
  expect(html).not.toContain('<time');
});
it('does not render receipts below AI messages or empty lists', () => {
  expect(renderToStaticMarkup(<MessageReceipts message={{ ...message, sender: 'claude' }}/>)).toBe('');
  expect(renderToStaticMarkup(<MessageReceipts message={{ ...message, receipts: [] }}/>)).toBe('');
});
