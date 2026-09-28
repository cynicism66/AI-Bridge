import { describe, expect, it } from 'vitest';
import { projectActions, displayPath } from './projectMenuLogic';
import { browserShortcut } from './browserPolicy';
describe('project context menus and browser policy', () => {
  it('offers the right operations and keeps deletion last', () => {
    expect(projectActions(false, true)).toEqual(['disable', 'open', 'forget', 'purge']);
    expect(projectActions(false, false)).toEqual(['enable', 'open', 'forget', 'purge']);
    expect(projectActions(true, false)).toEqual(['enable', 'open', 'hide']);
  });
  it('blocks browser shortcuts in release but preserves editing', () => {
    const key = (key: string, ctrlKey = false, altKey = false, shiftKey = false) => ({ key, ctrlKey, altKey, shiftKey, metaKey: false });
    for (const stroke of [key('F5'), key('F12'), key('r', true), key('R', true, false, true), key('p', true), key('ArrowLeft', false, true), key('I', true, false, true), key('BrowserBack')]) {
      expect(browserShortcut(stroke, true)).toBe(true);
      expect(browserShortcut(stroke, false)).toBe(false);
    }
    for (const char of ['a', 'c', 'v', 'x', 'z']) expect(browserShortcut(key(char, true), true)).toBe(false);
  });
  it('uses Windows paths without changing project keys', () => {
    const project = 'd:/bridge-demo';
    expect(displayPath(project + '\\AGENTS.md')).toBe('D:\\bridge-demo\\AGENTS.md');
    expect(displayPath('\\\\?\\UNC\\server\\share\\a')).toBe('\\\\server\\share\\a');
    expect(displayPath('\\\\?\\d:\\a')).toBe('D:\\a');
    expect(project).toBe('d:/bridge-demo');
  });
});
