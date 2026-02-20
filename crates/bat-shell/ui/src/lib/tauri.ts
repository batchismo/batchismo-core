import { invoke } from '@tauri-apps/api/core'
import type { Message, SessionMeta, PathPolicy, ToolInfo, BatConfig } from '../types'

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

export const deletePathPolicy = (path: string): Promise<void> =>
  invoke('delete_path_policy', { path })

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
