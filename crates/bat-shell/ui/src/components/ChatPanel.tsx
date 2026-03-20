import { useEffect, useRef, useState } from 'react'
import type { Message, AgentStatus, SubagentInfo } from '../types'
import { getSubagents } from '../lib/tauri'
import { MessageBubble } from './MessageBubble'
import { StreamingText } from './StreamingText'

interface Props {
  messages: Message[]
  streamingText: string
  agentStatus: AgentStatus
}

function SubagentStatusBar() {
  const [agents, setAgents] = useState<SubagentInfo[]>([])

  useEffect(() => {
    const poll = () => {
      getSubagents()
        .then(a => setAgents(a.filter(s => s.status === 'running')))
        .catch(() => {})
    }
    poll()
    const id = setInterval(poll, 3000)
    return () => clearInterval(id)
  }, [])

  if (agents.length === 0) return null

  return (
    <div className="mx-4 mb-2 px-3 py-2 rounded-lg bg-zinc-800/50 border border-zinc-700/50 text-xs text-zinc-400 flex items-center gap-2">
      <span className="animate-pulse">⏳</span>
      <span>{agents.length} worker{agents.length > 1 ? 's' : ''} running</span>
      <span className="text-zinc-600">—</span>
      <span className="truncate">{agents.map(a => a.label).join(', ')}</span>
    </div>
  )
}

export function ChatPanel({ messages, streamingText, agentStatus }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, streamingText])

  const isEmpty = messages.length === 0 && !streamingText

  return (
    <div className="flex-1 overflow-y-auto px-4 py-4 space-y-1">
      {isEmpty ? (
        <div className="flex h-full items-center justify-center">
          <div className="text-center text-zinc-600">
            <div className="text-4xl mb-3">&#9889;</div>
            <p className="text-lg font-medium text-zinc-500">Batchismo</p>
            <p className="text-sm mt-1">Your personal AI assistant</p>
          </div>
        </div>
      ) : (
        <>
          {messages.map(msg => (
            <MessageBubble key={msg.id} message={msg} />
          ))}
          {streamingText && (
            <div className="flex justify-start mb-3">
              <StreamingText text={streamingText} />
            </div>
          )}
          {agentStatus === 'thinking' && !streamingText && (
            <div className="flex justify-start mb-3">
              <div className="rounded-2xl bg-zinc-800 px-4 py-3 text-zinc-400 text-sm">
                <span className="animate-pulse">&bull;&bull;&bull;</span>
              </div>
            </div>
          )}
        </>
      )}
      <SubagentStatusBar />
      <div ref={bottomRef} />
    </div>
  )
}
