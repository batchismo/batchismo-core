import { invoke } from '@tauri-apps/api/core'
import type { Message, SessionMeta, PathPolicy, ToolInfo, BatConfig, AuditEntry, AuditFilter, AuditStats, MemoryFileInfo, Observation, ObservationFilter, ObservationSummary, SubagentInfo } from '../types'

export const sendMessage = (content: string): Promise<void> =>
  invoke('send_message', { content })

export const getHistory = (): Promise<Message[]> =>
  invoke('get_history')

export const getSession = (): Promise<SessionMeta> =>
  invoke('get_session')

export const getPathPolicies = (): Promise<PathPolicy[]> =>
  invoke('get_path_policies')

export const addPathPolicy = (path: string, access: string, recursive: boolean): Promise<void> =>
  invoke('add_path_policy', { path, access, recursive })

export const deletePathPolicy = (id: number): Promise<void> =>
  invoke('delete_path_policy', { id })

export const getTools = (): Promise<ToolInfo[]> =>
  invoke('get_tools')

export const toggleTool = (name: string, enabled: boolean): Promise<void> =>
  invoke('toggle_tool', { name, enabled })

export const getConfig = (): Promise<BatConfig> =>
  invoke('get_config')

export const updateConfig = (config: BatConfig): Promise<void> =>
  invoke('update_config', { config })

export const getSystemPrompt = (): Promise<string> =>
  invoke('get_system_prompt')

export const getAuditLogs = (filter: AuditFilter): Promise<AuditEntry[]> =>
  invoke('get_audit_logs', { filter })

export const getAuditStats = (): Promise<AuditStats> =>
  invoke('get_audit_stats')

// Memory
export const getMemoryFiles = (): Promise<MemoryFileInfo[]> =>
  invoke('get_memory_files')

export const getMemoryFile = (name: string): Promise<string> =>
  invoke('get_memory_file', { name })

export const updateMemoryFile = (name: string, content: string): Promise<void> =>
  invoke('update_memory_file', { name, content })

export const getObservations = (filter: ObservationFilter): Promise<Observation[]> =>
  invoke('get_observations', { filter })

export const getObservationSummary = (): Promise<ObservationSummary> =>
  invoke('get_observation_summary')

export const triggerConsolidation = (): Promise<string> =>
  invoke('trigger_consolidation')

// Subagents
export const getSubagents = (): Promise<SubagentInfo[]> =>
  invoke('get_subagents')

// Onboarding
export const isOnboardingComplete = (): Promise<boolean> =>
  invoke('is_onboarding_complete')

export const validateApiKey = (key: string): Promise<boolean> =>
  invoke('validate_api_key', { key })

export const completeOnboarding = (name: string, apiKey: string, folders: [string, string, boolean][]): Promise<void> =>
  invoke('complete_onboarding', { name, apiKey, folders })
