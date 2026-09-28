// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { zh } from './i18n/zh-CN';

vi.mock('./useWorkspace', () => ({
  useWorkspace: () => ({
    data: { projects: [{ project: 'd:/menu-demo', enabled: true, initialized: true, unread: 0 }], discovered: [{ project: 'd:/discovered-demo' }], global: true },
    project: null, tab: 'board', error: '', busy: false, refresh: 0,
    select: vi.fn(), run: vi.fn(), setTab: vi.fn(), reload: vi.fn(), setError: vi.fn(),
  }),
}));

let root: Root | undefined;
afterEach(async () => {
  await act(async () => root?.unmount());
  root = undefined;
  document.body.replaceChildren();
  vi.unstubAllGlobals();
});

describe.each([false, true])('page remains visible under portals (dark: %s)', dark => {
  it.each(['.project-row', '.discovered-row'])('%s does not copy page layout and restores focus on dismissal', async selector => {
    vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
    vi.stubGlobal('matchMedia', () => ({ matches: dark, addEventListener: vi.fn(), removeEventListener: vi.fn() }));
    const host = document.createElement('div');
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<App/>));
    const page = document.querySelector('.shell')!;
    const main = page.querySelector('main')!;
    const trigger = page.querySelector<HTMLElement>(selector)!;
    const content = main.textContent;
    await act(async () => {
      trigger.focus();
      trigger.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2 }));
    });
    expect(document.querySelector('[role="menu"]')).not.toBeNull();
    // FluentProvider copies its classes to portals; only the page can own the shell layout.
    expect(document.querySelectorAll('.shell')).toHaveLength(1);
    expect(main.textContent).toBe(content);
    const menu = document.querySelector<HTMLElement>('[role="menu"]')!;
    expect(menu.closest('.shell')).toBeNull();
    expect(menu.textContent).toContain(selector === '.project-row' ? zh.purgeProject : zh.hideProject);
    await act(async () => menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })));
    expect(document.querySelector('[role="menu"]')).toBeNull();
    expect(main.textContent).toBe(content);
    expect(document.activeElement).toBe(trigger);
  });
});
