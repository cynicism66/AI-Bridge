export interface ProjectSummary { project: string; enabled: boolean; initialized: boolean; unread: number; last: string | null }
export interface Snapshot { projects: ProjectSummary[]; discovered: { project: string }[]; global: boolean; revision: number }
export interface Agent { agent: string; role: string | null; enabled: boolean }
export interface Session { agent: string; session_no: number; branch: string; worktree: string; task: string; progress: string; blockers: string; next_step: string; updated_at: string }
export interface Claim { path: string; agent: string; session_no: number; note: string; expires_at: string }
export interface Detail { charter: { version: number; template: string; goal: string; summary: string } | null; agents: Agent[]; sessions: Session[]; claims: Claim[] }
export interface Message { id: number; project: string; sender: string; recipient: string; content: string; created_at: string; via: 'mcp' | 'cli' | 'gui' }
export interface MessagePage { messages: Message[]; before: number | null }
export type Tab = 'board' | 'chat' | 'history' | 'settings';
