import { zh } from './i18n/zh-CN';
import type { HistoryFilter, TemplateInfo } from './types';
export const lines = (s: string) => s.split(/\r?\n/).map(v => v.trim()).filter(Boolean);
export const validAgent = (s: string) => !!s.trim() && !/\s/.test(s.trim()) && !['human', 'bridge', 'all'].includes(s.trim().toLowerCase());
export function wizardError(step: number, participants: string[], template: TemplateInfo | null, assignments: Record<string,string>, goal: string): string {
  if (step === 0) return !participants.length ? zh.needParticipants : participants.every(validAgent) ? '' : zh.invalidAgent;
  if (step === 1) return template ? '' : zh.needTemplate;
  if (step === 2) {
    const selected = template?.roles.map(r => assignments[r.slot]) || [];
    if (!selected.length || selected.some(a => !a || !participants.includes(a)) || participants.length !== selected.length) return zh.needAssignments;
    return new Set(selected).size === selected.length ? '' : zh.duplicateAssignments;
  }
  return step === 3 && !goal.trim() ? zh.needGoal : '';
}
export const filterError = (f: HistoryFilter) => f.from && f.until && f.from > f.until ? zh.invalidDates : '';
export function permissionError(rules: string[], backendError: string | null): string {
  return backendError || (rules.some(r => !r.trim()) ? zh.permissionHelp : '');
}
