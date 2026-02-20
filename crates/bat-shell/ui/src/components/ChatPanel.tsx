import { useEffect, useRef } from 'react'
import type { Message, AgentStatus } from '../types'
import { MessageBubble } from './MessageBubble'
import { StreamingText } from './StreamingText'

interface Props {
  messages: Message[]
  streamingText: string
  agentStatus: AgentStatus
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
      <div ref={bottomRef} />
    </div>
  )
}
