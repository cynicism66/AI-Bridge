export type ProjectAction = 'enable' | 'disable' | 'open' | 'forget' | 'purge' | 'hide';
export function projectActions(discovered: boolean, enabled: boolean): ProjectAction[] {
  return discovered ? ['enable', 'open', 'hide'] : [enabled ? 'disable' : 'enable', 'open', 'forget', 'purge'];
}
export function displayPath(path: string): string {
  const clean = path.replace(/^\\\\\?\\UNC\\/i, '\\\\').replace(/^\\\\\?\\/, '');
  if (/^[a-z]:/i.test(clean)) return clean[0].toUpperCase() + clean.slice(1).replace(/\//g, '\\');
  return clean.startsWith('//') || clean.startsWith('\\\\') ? clean.replace(/\//g, '\\') : clean;
}
