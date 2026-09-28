// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { Composer, resizeComposer } from './Composer';

let host: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
const send = vi.fn<(content: string, to: string) => Promise<void>>();
const onError = vi.fn();
beforeEach(async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  send.mockReset().mockResolvedValue(undefined); onError.mockReset();
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  await act(async () => root.render(<Composer send={send} onError={onError}/>));
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.unstubAllGlobals(); });
async function type(value: string) {
  await act(async () => {
    const input = host.querySelector('textarea')!;
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')!.set!.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}
async function enter(options: KeyboardEventInit = {}) {
  const event = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true, ...options });
  await act(async () => { host.querySelector('textarea')!.dispatchEvent(event); });
  return event;
}
it('Enter sends once and clears the draft; Shift+Enter keeps native newline', async () => {
  await type('\u7b2c\u4e00\u884c');
  expect((await enter({ shiftKey: true })).defaultPrevented).toBe(false);
  expect(send).not.toHaveBeenCalled();
  expect((await enter()).defaultPrevented).toBe(true);
  expect(send).toHaveBeenCalledWith('\u7b2c\u4e00\u884c', 'all');
  expect(host.querySelector('textarea')!.value).toBe('');
});
it('IME Enter never sends, including keyCode 229 compatibility', async () => {
  await type('\u8f93\u5165\u6cd5');
  expect((await enter({ isComposing: true })).defaultPrevented).toBe(false);
  expect((await enter({ keyCode: 229 })).defaultPrevented).toBe(false);
  expect(send).not.toHaveBeenCalled();
});
it('empty and whitespace Enter neither sends nor inserts a newline', async () => {
  expect((await enter()).defaultPrevented).toBe(true);
  await type(' \n ');
  expect((await enter()).defaultPrevented).toBe(true);
  expect(send).not.toHaveBeenCalled();
});
it('in-flight sends cannot be repeated and failures preserve the draft', async () => {
  let reject!: (error: Error) => void;
  send.mockImplementation(() => new Promise((_, r) => { reject = r; }));
  await type('\u4fdd\u7559\u8349\u7a3f');
  await act(async () => {
    const input = host.querySelector('textarea')!;
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  });
  await enter();
  expect(send).toHaveBeenCalledTimes(1);
  await act(async () => reject(new Error('\u53d1\u9001\u5931\u8d25')));
  expect(onError).toHaveBeenCalledWith('Error: \u53d1\u9001\u5931\u8d25');
  expect(host.querySelector('textarea')!.value).toBe('\u4fdd\u7559\u8349\u7a3f');
});
it('height stays between three and eight lines and shrinks after content clears', () => {
  const input = host.querySelector('textarea')!;
  input.style.cssText = 'line-height:20px;padding:4px 8px;border:0;box-sizing:border-box';
  let height = 20;
  Object.defineProperty(input, 'scrollHeight', { get: () => height });
  resizeComposer(input); expect(input.style.height).toBe('68px');
  height = 108; resizeComposer(input); expect(input.style.height).toBe('108px');
  height = 400; resizeComposer(input); expect(input.style.height).toBe('168px');
  expect(input.style.overflowY).toBe('auto');
  height = 20; resizeComposer(input); expect(input.style.height).toBe('68px');
  expect(input.style.overflowY).toBe('hidden');
});
