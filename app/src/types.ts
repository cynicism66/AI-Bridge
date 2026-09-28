export interface ProjectSummary { project: string; enabled: boolean; initialized: boolean; unread: number; last: string | null }
export interface Snapshot { projects: ProjectSummary[]; discovered: { project: string }[]; global: boolean; revision: number }
export interface Agent { agent: string; role: string | null; enabled: boolean }
export interface Session { older: number; last_active: string; agent: string; session_no: number; branch: string; worktree: string; task: string; progress: string; blockers: string; next_step: string; updated_at: string }
export interface Claim { path: string; agent: string; session_no: number; note: string; expires_at: string }
export interface Detail { charter: { version: number; template: string; goal: string; summary: string } | null; agents: Agent[]; sessions: Session[]; claims: Claim[] }
export interface Message { id: number; project: string; sender: string; recipient: string; content: string; created_at: string; via: 'mcp' | 'cli' | 'gui'; receipts: ReadReceipt[] }
export interface MessagePage { messages: Message[]; before: number | null }
export type Tab = 'board' | 'chat' | 'history' | 'settings';

export interface Role { slot: string; duties: string; allow: string[]; deny: string[]; write: string[]; kickoff: boolean }
export interface TemplateInfo { name: string; charter: string; roles: Role[] }
export interface Management { known: string[]; roles: Role[]; assignments: Record<string,string>; charter: Detail['charter']; write_rules: boolean }
export interface HistoryFilter { agents: string[]; kinds: string[]; from: string; until: string; keyword: string }
export interface HistoryEvent { id: number; created_at: string; agent: string; kind: string; description: string; detail: unknown }
export interface HistoryPage { events: HistoryEvent[]; before: [string,number] | null }
export interface TransferPreview { id: number; preview: { body: string; summary: string; paths: string[]; remotes: string[]; released_claims: number } }
export interface Settings { preferences: { notifications_enabled: boolean; hidden_projects: string[]; close_tip_shown: boolean }; database: string; version: string }

export interface ReadReceipt { agent: string; read: boolean; read_at: string | null }
