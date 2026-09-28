import type { ReactNode } from 'react';
import { Dialog, DialogSurface, DialogBody, DialogTitle, DialogContent, DialogActions, Button, MessageBar, MessageBarBody } from '@fluentui/react-components';
import { zh } from './i18n/zh-CN';
export function Modal({ title, children, actions, close, busy = false, error = '' }: { title: string; children: ReactNode; actions?: ReactNode; close: () => void; busy?: boolean; error?: string }) {
  return <Dialog open onOpenChange={(_, d) => { if (!d.open && !busy) close(); }}>
    <DialogSurface className="management-modal"><DialogBody><DialogTitle>{title}</DialogTitle>
      <DialogContent>{error && <MessageBar intent="error"><MessageBarBody>{error}</MessageBarBody></MessageBar>}{children}</DialogContent>
      <DialogActions><Button disabled={busy} onClick={close}>{zh.cancel}</Button>{actions}</DialogActions>
    </DialogBody></DialogSurface>
  </Dialog>;
}
