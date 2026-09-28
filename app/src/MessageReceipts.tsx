import { actor } from './logic';
import { zh } from './i18n/zh-CN';
import type { Message } from './types';

export function MessageReceipts({ message }: { message: Message }) {
  if (message.sender !== 'human' || !message.receipts.length) return null;
  return <div className="message-receipts">{message.receipts.map((r, index) => <span key={r.agent}>
    {index > 0 && ' · '}{actor(r.agent)} {r.read ? zh.messageRead : zh.messageUnread}
    {r.read && r.read_at && <> <time title={r.read_at}>{r.read_at.slice(11, 16)}</time></>}
  </span>)}</div>;
}
