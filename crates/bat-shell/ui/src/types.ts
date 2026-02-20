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

// Settings types
export interface ToolInfo {
  name: string
  description: string
  enabled: boolean
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

export type AppView = 'chat' | 'settings'
export type SettingsPage = 'path-policies' | 'tools' | 'agent-config' | 'about'
