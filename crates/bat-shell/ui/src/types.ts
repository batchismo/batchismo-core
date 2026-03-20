export type Role = 'user' | 'assistant' | 'system'
export type SessionStatus = 'active' | 'idle' | 'completed' | 'failed'
export type AgentStatus = 'idle' | 'thinking' | 'tool_calling'

export interface ImageAttachment {
  data: string       // base64-encoded image data
  mediaType: string  // MIME type, e.g. "image/png"
}

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
  images: ImageAttachment[]
  tool_calls: ToolCall[]
  tool_results: ToolResult[]
  created_at: string
  token_input: number | null
  token_output: number | null
  // Question from sub-agent
  question?: {
    question_id: string
    question: string
    context: string
    blocking: boolean
    answered?: boolean
  }
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
  id?: number
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
  | { type: 'Question'; question_id: string; question: string; context: string; blocking: boolean }
  | { type: 'Progress'; summary: string; percent?: number }

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
  exec_run: { name: 'Execute', icon: '▶️' },
  exec_output: { name: 'Get Output', icon: '📋' },
  exec_write: { name: 'Write Input', icon: '⌨️' },
  exec_kill: { name: 'Kill Process', icon: '🛑' },
  exec_list: { name: 'List Processes', icon: '📊' },
  app_open: { name: 'Open App', icon: '🚀' },
  system_info: { name: 'System Info', icon: '💻' },
  session_spawn: { name: 'Spawn Subagent', icon: '🔀' },
  session_status: { name: 'Subagent Status', icon: '📡' },
}

export interface AgentConfig {
  name: string
  model: string
  thinking_level: string
  api_key: string | null
  personality_prompt: string | null
  disabled_tools: string[]
  enabled_models: string[]
}

export interface TelegramChannelConfig {
  enabled: boolean
  bot_token: string
  allow_from: number[]
}

export interface ChannelsConfig {
  telegram?: TelegramChannelConfig
}

export interface ApiKeys {
  anthropic: string | null
  openai: string | null
  elevenlabs: string | null
  ollama_endpoint: string | null
}

export interface OllamaModel {
  name: string
  size: number
  modifiedAt: string | null
  parameterSize: string | null
}

export interface VoiceConfig {
  tts_enabled: boolean
  tts_provider: string
  openai_voice: string
  openai_tts_model: string
  elevenlabs_voice_id: string | null
  stt_enabled: boolean
}

export interface BatConfig {
  agent: AgentConfig
  gateway: { port: number; log_level: string }
  memory: { update_mode: string; consolidation_schedule: string; max_memory_file_size_kb: number }
  sandbox: { memory_limit_mb: number; cpu_shares: number; max_concurrent_subagents: number; subagent_timeout_minutes: number }
  paths: PathPolicy[]
  channels?: ChannelsConfig
  voice: VoiceConfig
  api_keys: ApiKeys
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

export type DiffKind = 'added' | 'removed' | 'context'

export interface DiffLine {
  kind: DiffKind
  content: string
}

// Usage types
export interface UsageStats {
  totalInput: number
  totalOutput: number
  sessions: SessionUsage[]
  byModel: ModelUsage[]
  estimatedCostUsd: number
}

export interface SessionUsage {
  key: string
  model: string
  tokenInput: number
  tokenOutput: number
  messageCount: number
  lastActive: string
}

export interface ModelUsage {
  model: string
  tokenInput: number
  tokenOutput: number
  sessionCount: number
}

// Subagent types
export type SubagentStatus = 'running' | 'waiting_for_answer' | 'paused' | 'completed' | 'failed' | 'cancelled' | 'timed_out' | 'archived'

export interface SubagentInfo {
  sessionId: string
  sessionKey: string
  label: string
  task: string
  status: SubagentStatus
  startedAt: string
  completedAt: string | null
  summary: string | null
  tokenInput: number
  tokenOutput: number
  progress?: SubagentProgress
  pendingQuestion?: PendingQuestion
}

export interface SubagentProgress {
  summary: string
  percent?: number
}

export interface PendingQuestion {
  questionId: string
  question: string
  context: string
  blocking: boolean
}

// ElevenLabs voice (fetched from API)
export interface ElevenLabsVoice {
  voiceId: string
  name: string
  category: string
  previewUrl: string | null
}

export type AppView = 'chat' | 'settings' | 'logs' | 'memory' | 'activity' | 'usage'

// Onboarding
export interface FolderAccess {
  path: string
  access: string
  recursive: boolean
}
export type SettingsPage = 'api-keys' | 'path-policies' | 'tools' | 'agent-config' | 'personality' | 'channels' | 'voice' | 'audit' | 'sandbox' | 'ollama' | 'about'
