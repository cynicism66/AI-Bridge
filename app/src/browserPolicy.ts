export interface KeyStroke { key: string; ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }
export function browserShortcut(e: KeyStroke, release: boolean): boolean {
  if (!release) return false;
  const key = e.key.toLowerCase();
  return ['f5', 'f12', 'browserback', 'browserforward', 'browserrefresh', 'browserhome', 'browsersearch'].includes(key)
    || ((e.ctrlKey || e.metaKey) && ['r', 'p', 'u', 's', 'o', 'l', 'n', 't', 'w'].includes(key))
    || ((e.ctrlKey || e.metaKey) && e.shiftKey && ['i', 'j', 'c'].includes(key))
    || (e.altKey && ['arrowleft', 'arrowright', 'home'].includes(key));
}
export function installBrowserPolicy(release: boolean) {
  // Cancel browser defaults after React has handled the project menu.
  document.addEventListener('contextmenu', e => e.preventDefault());
  document.addEventListener('keydown', e => {
    if (browserShortcut(e, release)) { e.preventDefault(); e.stopPropagation(); }
  }, true);
  document.addEventListener('auxclick', e => { if (release && e.button >= 3) e.preventDefault(); }, true);
}
