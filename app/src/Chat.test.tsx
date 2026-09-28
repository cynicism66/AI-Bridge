// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { Chat } from './Chat';
import { zh } from './i18n/zh-CN';
import type { Message } from './types';

const api = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => api);

it('refreshes read receipts on older loaded pages and keeps IDs and guidance visible', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  vi.stubGlobal('requestAnimationFrame', (f: () => void) => { f(); return 0; });
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  let read = false;
  let newest = 2;
  const message = (id: number): Message => ({ id, project: '/test', sender: 'human', recipient: 'claude', content: `message ${id}`, created_at: '', via: 'gui', receipts: [{ agent: 'claude', read, read_at: read ? '2090-01-02 16:10:20' : null }] });
  api.invoke.mockImplementation(async (command, args) => command === 'message_page' ? args.before ? { messages: [1, 2].filter(id => id < args.before).map(message), before: null } : { messages: [message(newest)], before: newest } : undefined);
  const reload = vi.fn(async () => {});
  const onError = vi.fn();
  try {
    await act(async () => root.render(<Chat project="/test" refresh={0} reload={reload} onError={onError}/>));
    newest = 3;
    await act(async () => root.render(<Chat project="/test" refresh={1} reload={reload} onError={onError}/>));
    expect([...host.querySelectorAll('.message-id')].map(n => n.textContent)).toEqual(['#2', '#3']);
    await act(async () => host.querySelector<HTMLButtonElement>('.older')!.click());
    expect(host.querySelectorAll('.message-id')).toHaveLength(3);
    expect(host.querySelector('.message-id')!.textContent).toBe('#1');
    expect(host.querySelector('textarea')!.placeholder).toBe(zh.messagePlaceholder);
    expect(host.querySelector('.message-receipts')!.textContent).toContain(zh.messageUnread);
    read = true;
    await act(async () => root.render(<Chat project="/test" refresh={2} reload={reload} onError={onError}/>));
    expect(host.querySelector('.message-receipts')!.textContent).toContain(`${zh.messageRead} 16:10`);
    expect(host.querySelectorAll('.message')).toHaveLength(3);
    expect(onError).not.toHaveBeenCalled();
  } finally {
    await act(async () => root.unmount());
    host.remove();
    vi.unstubAllGlobals();
  }
});
