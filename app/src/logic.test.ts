import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { zh } from './i18n/zh-CN';
import { actor, historyMessageId, mergeMessages, projectName, source, trayState, unreadTotal } from './logic';
import type { Message } from './types';
const message = (id: number, recipient = 'all', sender = 'codex'): Message => ({ id, project: '/test', sender, recipient, content: 'test', created_at: '', via: 'mcp', receipts: [] });
describe('presentation logic', () => {
  it('unread counts and global-off precedence', () => {
    expect(unreadTotal([{ unread: 2 }, { unread: 3 }, { unread: -1 }])).toBe(5);
    expect(trayState(false, 5)).toBe('off');
    expect(trayState(true, 0)).toBe('normal');
    expect(trayState(true, 2)).toBe('unread');
  });
  it('merges pages without duplicates or reordering', () => {
    expect(mergeMessages([message(3), message(5)], [message(2), { ...message(5), content: 'latest' }])).toEqual([message(2), message(3), { ...message(5), content: 'latest' }]);
  });
  it('labels paths and provenance without assuming drive letters', () => {
    expect(projectName('D:\\Code\\Project\\')).toBe('Project');
    expect(projectName('//server/share')).toBe('share');
    expect(source('gui')).toBe(zh.sourceGui);
    expect(source('cli')).toBe(zh.sourceCli);
    expect(actor('human')).toBe(zh.human);
  });
});
describe('centralized resources', () => {
  it('all frontend and native resource references exist and components have no inline Chinese', () => {
    const root = fileURLToPath(new URL('.', import.meta.url));
    const resources = new Set(Object.keys(zh));
    for (const name of readdirSync(root).filter(n => /\.tsx?$/.test(n) && !n.endsWith('.test.ts'))) {
      const code = readFileSync(join(root, name), 'utf8');
      for (const match of code.matchAll(/zh\.(\w+)/g)) expect(resources.has(match[1]), `${name}: ${match[1]}`).toBe(true);
      expect(/[\u3400-\u9fff]/.test(code), name).toBe(false);
    }
    const native = join(root, '../src-tauri/src');
    for (const name of readdirSync(native).filter(n => n.endsWith('.rs'))) {
      for (const match of readFileSync(join(native, name), 'utf8').matchAll(/text\("(\w+)"\)/g)) expect(resources.has(match[1]), `${name}: ${match[1]}`).toBe(true);
    }
    for (const value of Object.values(zh)) expect(value.length).toBeGreaterThan(0);
  });
});

it('uses the message identity rather than the history event identity', () => {
  expect(historyMessageId({ message_id: 141 })).toBe(141);
  for (const detail of [null, {}, { id: 42 }, { message_id: '141' }, { message_id: -1 }]) expect(historyMessageId(detail)).toBeNull();
});

it('pending actions override unread red while global off stays gray',()=>{expect(trayState(true,3,1)).toBe('attention');expect(trayState(false,3,1)).toBe('off');expect(trayState(true,3,0)).toBe('unread');});
