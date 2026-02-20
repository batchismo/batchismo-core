import { useState, useCallback, useEffect } from 'react'
import type { Message, AgentStatus, BatEvent } from '../types'
import { sendMessage, getHistory } from '../lib/tauri'
import { useBatEvents } from './useBatEvents'

export function useChat() {
  const [messages, setMessages] = useState<Message[]>([])
  const [streamingText, setStreamingText] = useState('')
  const [agentStatus, setAgentStatus] = useState<AgentStatus>('idle')
  const [error, setError] = useState<string | null>(null)

  // Load history on mount
  useEffect(() => {
    getHistory().then(setMessages).catch(console.error)
  }, [])

  const handleEvent = useCallback((event: BatEvent) => {
    switch (event.type) {
      case 'TextDelta':
        setAgentStatus('thinking')
        setStreamingText(prev => prev + event.content)
        break
      case 'ToolCallStart':
        setAgentStatus('tool_calling')
        break
      case 'ToolCallResult':
        setAgentStatus('thinking')
        break
      case 'TurnComplete':
        setMessages(prev => [...prev, event.message])
        setStreamingText('')
        setAgentStatus('idle')
        break
      case 'Error':
        setError(event.message)
        setStreamingText('')
        setAgentStatus('idle')
        break
    }
  }, [])

  useBatEvents(handleEvent)

  const send = useCallback(async (content: string) => {
    if (agentStatus !== 'idle') return
    setError(null)
    // Optimistically add user message
    const userMsg: Message = {
      id: crypto.randomUUID(),
      session_id: '',
      role: 'user',
      content,
      tool_calls: [],
      tool_results: [],
      created_at: new Date().toISOString(),
      token_input: null,
      token_output: null,
    }
    setMessages(prev => [...prev, userMsg])
    setAgentStatus('thinking')
    try {
      await sendMessage(content)
    } catch (e) {
      setError(String(e))
      setAgentStatus('idle')
    }
  }, [agentStatus])

  return { messages, streamingText, agentStatus, error, send }
}
