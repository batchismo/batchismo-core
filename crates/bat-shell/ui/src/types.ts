export type Role = 'user' | 'assistant' | 'system'
export type SessionStatus = 'active' | 'idle' | 'completed' | 'failed'
export type AgentStatus = 'idle' | 'thinking' | 'tool_calling'

export interface ToolCall {
  id: string
  name: string
  input: unknown
}

export interface ToolResult {
  tool_call_id: string
  content: string
  is_error: boolean
}

export interface Message {
  id: string
  session_id: string
  role: Role
  content: string
  tool_calls: ToolCall[]
  tool_results: ToolResult[]
  created_at: string
  token_input: number | null
  token_output: number | null
}

export interface SessionMeta {
  id: string
  key: string
  model: string
  status: SessionStatus
  created_at: string
  updated_at: string
  token_input: number
  token_output: number
}

export interface PathPolicy {
  path: string
  access: 'read-only' | 'read-write' | 'write-only'
  recursive: boolean
  description: string | null
}

// Tauri bat-event payload types
export type BatEvent =
  | { type: 'TextDelta'; content: string }
  | { type: 'ToolCallStart'; tool_call: ToolCall }
  | { type: 'ToolCallResult'; result: ToolResult }
  | { type: 'TurnComplete'; message: Message }
  | { type: 'Error'; message: string }
  | { type: 'AuditLog'; level: string; category: string; event: string; summary: string; detail_json: string | null }

// Settings types
export interface ToolInfo {
  name: string
  displayName: string
  description: string
  icon: string
  enabled: boolean
}

// Tool display name mapping for chat UI
export const TOOL_DISPLAY: Record<string, { name: string; icon: string }> = {
  fs_read: { name: 'Read File', icon: '📄' },
  fs_write: { name: 'Write File', icon: '✏️' },
  fs_list: { name: 'List Directory', icon: '📁' },
  web_fetch: { name: 'Fetch URL', icon: '🌐' },
  shell_run: { name: 'Run Command', icon: '⚡' },
}

export interface AgentConfig {
  name: string
  model: string
  thinking_level: string
  api_key: string | null
  disabled_tools: string[]
}

export interface BatConfig {
  agent: AgentConfig
  gateway: { port: number; log_level: string }
  memory: { update_mode: string; consolidation_schedule: string; max_memory_file_size_kb: number }
  sandbox: { memory_limit_mb: number; cpu_shares: number; max_concurrent_subagents: number }
  paths: PathPolicy[]
}

// Audit log types
export type AuditLevel = 'debug' | 'info' | 'warn' | 'error'
export type AuditCategory = 'agent' | 'tool' | 'gateway' | 'ipc' | 'config'

export interface AuditEntry {
  id: number
  ts: string
  sessionId: string | null
  level: AuditLevel
  category: AuditCategory
  event: string
  summary: string
  detailJson: string | null
}

export interface AuditFilter {
  level?: AuditLevel | null
  category?: AuditCategory | null
  sessionId?: string | null
  since?: string | null
  until?: string | null
  search?: string | null
  limit?: number | null
  offset?: number | null
}

export interface AuditStats {
  total: number
  byLevel: { debug: number; info: number; warn: number; error: number }
  byCategory: { agent: number; tool: number; gateway: number; ipc: number; config: number }
}

// Memory types
export type ObservationKind = 'tool_use' | 'path_access' | 'user_correction' | 'task_pattern' | 'preference'

export interface Observation {
  id: number
  ts: string
  sessionId: string | null
  kind: ObservationKind
  key: string
  value: string | null
  count: number
}

export interface ObservationFilter {
  kind?: ObservationKind | null
  since?: string | null
  key?: string | null
  limit?: number | null
}

export interface ObservationSummary {
  totalObservations: number
  totalSessions: number
  topTools: [string, number][]
  topPaths: [string, number][]
  lastConsolidation: string | null
}

export interface MemoryFileInfo {
  name: string
  sizeBytes: number
  modifiedAt: string | null
}

export type AppView = 'chat' | 'settings' | 'logs' | 'memory'

// Onboarding
export interface FolderAccess {
  path: string
  access: string
  recursive: boolean
}
export type SettingsPage = 'path-policies' | 'tools' | 'agent-config' | 'about'
