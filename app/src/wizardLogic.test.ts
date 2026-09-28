import { describe, expect, it } from 'vitest';
import { filterError, lines, permissionError, validAgent, wizardError } from './wizardLogic';
import { zh } from './i18n/zh-CN';
import type { TemplateInfo } from './types';
const template: TemplateInfo = { name: 'pair', charter: '', roles: ['a','b'].map(slot => ({ slot, duties: '', allow: [], deny: [], write: [], kickoff: slot === 'a' })) };
describe('wizard and management validation', () => {
  it('requires selected participants, valid template and nonempty goal', () => {
    expect(wizardError(0, [], null, {}, '')).toBe(zh.needParticipants);
    expect(validAgent('human')).toBe(false);
    expect(validAgent('two words')).toBe(false);
    expect(validAgent('agent-three')).toBe(true);
    expect(wizardError(1, ['claude'], null, {}, '')).toBe(zh.needTemplate);
    expect(wizardError(3, ['claude'], template, {}, '  ')).toBe(zh.needGoal);
  });
  it('requires every role and each selected AI exactly once', () => {
    expect(wizardError(2, ['claude','codex'], template, { a: 'claude' }, '')).toBe(zh.needAssignments);
    expect(wizardError(2, ['claude','codex'], template, { a: 'claude', b: 'claude' }, '')).toBe(zh.duplicateAssignments);
    expect(wizardError(2, ['claude','codex'], template, { a: 'claude', b: 'codex' }, '')).toBe('');
    expect(wizardError(2, ['claude','codex','third'], template, { a: 'claude', b: 'codex' }, '')).toBe(zh.needAssignments);
  });
  it('preserves backend write-rule errors and parses multiline lists', () => {
    expect(lines('docs/**\r\n \n AGENTS.md ')).toEqual(['docs/**','AGENTS.md']);
    expect(permissionError(['../x'], 'invalid path')).toBe('invalid path');
    expect(permissionError([], null)).toBe('');
  });
  it('accepts open date ranges and rejects reversed history filters', () => {
    const f = { agents: ['claude','codex'], kinds: ['message'], keyword: 'hello', from: '2026-09-28', until: '' };
    expect(filterError(f)).toBe('');
    expect(filterError({ ...f, until: '2026-09-27' })).toBe(zh.invalidDates);
    expect(filterError({ ...f, until: f.from })).toBe('');
  });
});
