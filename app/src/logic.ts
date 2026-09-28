import type { Message, ProjectSummary } from './types';
import { zh } from './i18n/zh-CN';
export function projectName(path: string): string { return path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() || path; }
export function unreadTotal(projects: Pick<ProjectSummary, 'unread'>[]): number { return projects.reduce((n, p) => n + Math.max(0, p.unread), 0); }
export function trayState(global: boolean, unread: number): 'off' | 'normal' | 'unread' { return !global ? 'off' : unread > 0 ? 'unread' : 'normal'; }
export function mergeMessages(old: Message[], latest: Message[]): Message[] {
  return [...new Map([...old, ...latest].map(m => [m.id, m])).values()].sort((a, b) => a.id - b.id);
}
export function actor(name: string): string { return name === 'human' ? zh.human : name === 'all' ? zh.all : name === 'bridge' ? zh.bridge : name; }
export function source(via: Message['via']): string { return via === 'gui' ? zh.sourceGui : via === 'cli' ? zh.sourceCli : zh.sourceMcp; }

export function historyMessageId(detail: unknown): number | null {
  if (!detail || typeof detail !== 'object' || !('message_id' in detail)) return null;
  const id = detail.message_id;
  return typeof id === 'number' && Number.isSafeInteger(id) && id > 0 ? id : null;
}
