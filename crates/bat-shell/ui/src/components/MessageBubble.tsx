import ReactMarkdown from 'react-markdown'
import type { Message } from '../types'
import { ToolCallBlock } from './ToolCallBlock'

interface Props {
  message: Message
}

export function MessageBubble({ message }: Props) {
  const isUser = message.role === 'user'

  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'} mb-3`}>
      <div className={`max-w-[85%] ${isUser ? 'order-2' : 'order-1'}`}>
        {/* Tool calls */}
        {message.tool_calls.length > 0 && (
          <div className="mb-2">
            {message.tool_calls.map((tc) => {
              const result = message.tool_results.find(r => r.tool_call_id === tc.id)
              return <ToolCallBlock key={tc.id} toolCall={tc} result={result} />
            })}
          </div>
        )}
        {/* Image attachments */}
        {message.images && message.images.length > 0 && (
          <div className="flex gap-2 mb-2 flex-wrap">
            {message.images.map((img, i) => (
              <img
                key={i}
                src={`data:${img.mediaType};base64,${img.data}`}
                alt="attached image"
                className="max-h-48 max-w-64 rounded-lg border border-zinc-600 object-contain"
              />
            ))}
          </div>
        )}
        {/* Message content */}
        {message.content && (
          <div
            className={`rounded-2xl px-4 py-3 text-sm leading-relaxed ${
              isUser
                ? 'bg-zinc-700 text-zinc-100'
                : 'text-[#39FF14]'
            }`}
          >
            {isUser ? (
              <p className="whitespace-pre-wrap">{message.content}</p>
            ) : (
              <div className="prose prose-invert prose-sm max-w-none">
                <ReactMarkdown>{message.content}</ReactMarkdown>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
