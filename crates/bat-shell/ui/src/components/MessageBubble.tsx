import ReactMarkdown from 'react-markdown'
import { useState } from 'react'
import type { Message } from '../types'
import { ToolCallBlock } from './ToolCallBlock'
import { sendMessage } from '../lib/tauri'

const ORCHESTRATION_TOOLS = new Set([
  'session_spawn', 'session_status', 'session_pause',
  'session_resume', 'session_instruct', 'session_cancel', 'session_answer'
])

interface Props {
  message: Message
}

export function MessageBubble({ message }: Props) {
  const isUser = message.role === 'user'
  const [answerText, setAnswerText] = useState('')
  const [isAnswering, setIsAnswering] = useState(false)

  const handleAnswerSubmit = async () => {
    if (!message.question || !answerText.trim()) return

    setIsAnswering(true)
    try {
      // Send answer via session_answer tool
      await sendMessage(`session_answer ${message.session_id} ${message.question.question_id} ${answerText}`)
      setAnswerText('')
    } catch (error) {
      console.error('Error sending answer:', error)
    } finally {
      setIsAnswering(false)
    }
  }

  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'} mb-3`}>
      <div className={`max-w-[85%] ${isUser ? 'order-2' : 'order-1'}`}>
        {/* Tool calls */}
        {message.tool_calls.length > 0 && (
          <div className="mb-2">
            {message.tool_calls.map((tc) => {
              const result = message.tool_results.find(r => r.tool_call_id === tc.id)
              if (ORCHESTRATION_TOOLS.has(tc.name)) {
                // Compact one-liner for orchestration tools
                const icon = tc.name === 'session_spawn' ? '🔀' : tc.name === 'session_status' ? '📡' : '🔗'
                const label = tc.name === 'session_spawn'
                  ? `Spawned: "${(tc.input as any)?.label || (tc.input as any)?.task?.slice(0, 40) || 'worker'}"`
                  : tc.name === 'session_status'
                  ? 'Checking workers...'
                  : tc.name
                return (
                  <div key={tc.id} className="text-xs text-zinc-500 py-0.5 px-1">
                    {icon} {label}
                  </div>
                )
              }
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

        {/* Question bubble */}
        {message.question && (
          <div className="mb-2 border border-yellow-500/30 bg-yellow-500/10 rounded-2xl p-4">
            <div className="flex items-start gap-2 mb-2">
              <span className="text-yellow-500 text-lg">❓</span>
              <div className="flex-1">
                <div className="text-xs text-yellow-400 font-semibold uppercase tracking-wider mb-1">
                  Sub-agent Question {message.question.blocking ? '(Blocking)' : '(Non-blocking)'}
                </div>
                <p className="text-sm text-yellow-100 mb-2">{message.question.question}</p>
                {message.question.context && (
                  <p className="text-xs text-yellow-200/70 mb-3">Context: {message.question.context}</p>
                )}
                {!message.question.answered && (
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={answerText}
                      onChange={(e) => setAnswerText(e.target.value)}
                      placeholder="Type your answer..."
                      className="flex-1 px-3 py-1.5 bg-zinc-800 border border-zinc-600 rounded text-sm text-zinc-100 placeholder-zinc-500 focus:border-yellow-500 focus:outline-none"
                      onKeyPress={(e) => e.key === 'Enter' && handleAnswerSubmit()}
                      disabled={isAnswering}
                    />
                    <button
                      onClick={handleAnswerSubmit}
                      disabled={!answerText.trim() || isAnswering}
                      className="px-3 py-1.5 bg-yellow-600 hover:bg-yellow-700 disabled:bg-yellow-600/50 text-yellow-100 text-sm rounded transition-colors"
                    >
                      {isAnswering ? '...' : 'Answer'}
                    </button>
                  </div>
                )}
                {message.question.answered && (
                  <div className="text-xs text-green-400 font-medium">✓ Answered</div>
                )}
              </div>
            </div>
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
