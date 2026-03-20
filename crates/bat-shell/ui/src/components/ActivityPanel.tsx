import { useEffect, useState } from 'react'
import type { SubagentInfo } from '../types'
import { getSubagents } from '../lib/tauri'

const STATUS_COLORS: Record<string, string> = {
  running: 'text-blue-400 bg-blue-400/10 border-blue-400/30',
  waiting_for_answer: 'text-yellow-400 bg-yellow-400/10 border-yellow-400/30',
  paused: 'text-purple-400 bg-purple-400/10 border-purple-400/30',
  completed: 'text-emerald-400 bg-emerald-400/10 border-emerald-400/30',
  failed: 'text-red-400 bg-red-400/10 border-red-400/30',
  cancelled: 'text-zinc-400 bg-zinc-400/10 border-zinc-400/30',
  timed_out: 'text-orange-400 bg-orange-400/10 border-orange-400/30',
  archived: 'text-zinc-500 bg-zinc-500/10 border-zinc-500/30',
}

const STATUS_ICONS: Record<string, string> = {
  running: '⏳',
  waiting_for_answer: '❓',
  paused: '⏸️',
  completed: '✅',
  failed: '❌',
  cancelled: '⏹',
  timed_out: '⏰',
  archived: '📦',
}

function timeAgo(isoDate: string): string {
  const ms = Date.now() - new Date(isoDate).getTime()
  const sec = Math.floor(ms / 1000)
  if (sec < 60) return `${sec}s ago`
  const min = Math.floor(sec / 60)
  if (min < 60) return `${min}m ago`
  const hr = Math.floor(min / 60)
  return `${hr}h ${min % 60}m ago`
}

export function ActivityPanel() {
  const [subagents, setSubagents] = useState<SubagentInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [expandedId, setExpandedId] = useState<string | null>(null)

  const refresh = () => {
    getSubagents()
      .then(setSubagents)
      .catch(console.error)
      .finally(() => setLoading(false))
  }

  useEffect(() => {
    refresh()
    // Auto-refresh every 5 seconds if any are running
    const interval = setInterval(() => {
      getSubagents().then(agents => {
        setSubagents(agents)
        // Stop polling if none are running
      }).catch(console.error)
    }, 5000)
    return () => clearInterval(interval)
  }, [])

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
        Loading activity...
      </div>
    )
  }

  if (subagents.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center text-zinc-500 gap-3 px-8">
        <div className="text-4xl">🔀</div>
        <div className="text-sm text-center">
          <p className="font-medium text-zinc-400 mb-1">No subagents yet</p>
          <p className="text-xs text-zinc-600">
            When the agent spawns background tasks, they'll appear here.
            Use <code className="text-zinc-400 bg-zinc-800 px-1 rounded">session_spawn</code> in chat.
          </p>
        </div>
      </div>
    )
  }

  const running = subagents.filter(s => s.status === 'running')
  const done = subagents.filter(s => s.status !== 'running')

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      {/* Summary bar */}
      <div className="flex items-center gap-3 text-xs text-zinc-500">
        <span>{subagents.length} total</span>
        {running.length > 0 && (
          <span className="text-blue-400">{running.length} running</span>
        )}
        <button
          onClick={refresh}
          className="ml-auto text-zinc-500 hover:text-zinc-300 transition-colors"
          title="Refresh"
        >
          ↻ Refresh
        </button>
      </div>

      {/* Running section */}
      {running.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">Running</h3>
          {running.map(agent => (
            <SubagentCard
              key={agent.sessionId}
              agent={agent}
              expanded={expandedId === agent.sessionId}
              onToggle={() => setExpandedId(expandedId === agent.sessionId ? null : agent.sessionId)}
            />
          ))}
        </div>
      )}

      {/* Completed section */}
      {done.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">History</h3>
          {done.map(agent => (
            <SubagentCard
              key={agent.sessionId}
              agent={agent}
              expanded={expandedId === agent.sessionId}
              onToggle={() => setExpandedId(expandedId === agent.sessionId ? null : agent.sessionId)}
            />
          ))}
        </div>
      )}
    </div>
  )
}

function SubagentCard({ agent, expanded, onToggle }: {
  agent: SubagentInfo
  expanded: boolean
  onToggle: () => void
}) {
  const statusClass = STATUS_COLORS[agent.status] || STATUS_COLORS.cancelled
  const icon = STATUS_ICONS[agent.status] || '❓'

  const handleLifecycleAction = (action: string) => {
    // Send tool call through chat to manage subagent lifecycle
    const message = `session_${action} ${agent.sessionKey}`
    import('../lib/tauri').then(({ sendMessage }) => {
      sendMessage(message)
    })
  }

  return (
    <div
      className={`border rounded-lg bg-zinc-900/50 overflow-hidden transition-colors ${
        agent.status === 'running' ? 'border-blue-500/20' : 
        agent.status === 'waiting_for_answer' ? 'border-yellow-500/20' :
        agent.status === 'paused' ? 'border-purple-500/20' :
        'border-zinc-800'
      }`}
    >
      <button
        onClick={onToggle}
        className="w-full flex items-center gap-3 px-3 py-2.5 text-left hover:bg-zinc-800/50 transition-colors"
      >
        <span className="text-base">{icon}</span>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-zinc-200 truncate">
              {agent.label}
            </span>
            <span className={`text-[10px] px-1.5 py-0.5 rounded-full border font-medium ${statusClass}`}>
              {agent.status.replace('_', ' ')}
            </span>
          </div>
          <div className="text-xs text-zinc-500 mt-0.5">
            {timeAgo(agent.startedAt)}
            {agent.tokenInput + agent.tokenOutput > 0 && (
              <span className="ml-2">
                {agent.tokenInput + agent.tokenOutput} tokens
              </span>
            )}
          </div>
          {/* Progress indicator */}
          {agent.progress && (
            <div className="text-xs text-blue-400 mt-1">
              {agent.progress.summary}
              {agent.progress.percent && (
                <span className="ml-1 text-zinc-500">({Math.round(agent.progress.percent)}%)</span>
              )}
            </div>
          )}
          {/* Question indicator */}
          {agent.pendingQuestion && (
            <div className="text-xs text-yellow-400 mt-1 flex items-center gap-1">
              <span>❓</span>
              <span>Waiting for answer</span>
            </div>
          )}
        </div>
        <svg
          className={`w-4 h-4 text-zinc-500 transition-transform ${expanded ? 'rotate-180' : ''}`}
          fill="none" viewBox="0 0 24 24" stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {expanded && (
        <div className="px-3 pb-3 border-t border-zinc-800 pt-2 space-y-2">
          {/* Lifecycle control buttons */}
          {(agent.status === 'running' || agent.status === 'waiting_for_answer') && (
            <div className="flex gap-1">
              <button
                onClick={() => handleLifecycleAction('pause')}
                className="text-xs px-2 py-1 bg-purple-600/20 text-purple-400 border border-purple-600/30 rounded hover:bg-purple-600/30 transition-colors"
                title="Pause subagent"
              >
                ⏸️ Pause
              </button>
              <button
                onClick={() => handleLifecycleAction('cancel')}
                className="text-xs px-2 py-1 bg-red-600/20 text-red-400 border border-red-600/30 rounded hover:bg-red-600/30 transition-colors"
                title="Cancel subagent"
              >
                ❌ Cancel
              </button>
            </div>
          )}
          {agent.status === 'paused' && (
            <div className="flex gap-1">
              <button
                onClick={() => handleLifecycleAction('resume')}
                className="text-xs px-2 py-1 bg-blue-600/20 text-blue-400 border border-blue-600/30 rounded hover:bg-blue-600/30 transition-colors"
                title="Resume subagent"
              >
                ▶️ Resume
              </button>
              <button
                onClick={() => handleLifecycleAction('cancel')}
                className="text-xs px-2 py-1 bg-red-600/20 text-red-400 border border-red-600/30 rounded hover:bg-red-600/30 transition-colors"
                title="Cancel subagent"
              >
                ❌ Cancel
              </button>
            </div>
          )}

          <div>
            <span className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Task</span>
            <p className="text-xs text-zinc-300 mt-0.5 whitespace-pre-wrap">{agent.task}</p>
          </div>
          
          {agent.progress && (
            <div>
              <span className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Progress</span>
              <p className="text-xs text-blue-400 mt-0.5">{agent.progress.summary}</p>
              {agent.progress.percent && (
                <div className="w-full bg-zinc-800 rounded-full h-1.5 mt-1">
                  <div 
                    className="bg-blue-500 h-1.5 rounded-full transition-all"
                    style={{ width: `${agent.progress.percent}%` }}
                  />
                </div>
              )}
            </div>
          )}

          {agent.pendingQuestion && (
            <div>
              <span className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Question</span>
              <p className="text-xs text-yellow-300 mt-0.5 whitespace-pre-wrap">{agent.pendingQuestion.question}</p>
              {agent.pendingQuestion.context && (
                <p className="text-xs text-zinc-400 mt-1 whitespace-pre-wrap">{agent.pendingQuestion.context}</p>
              )}
            </div>
          )}

          {agent.summary && (
            <div>
              <span className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Summary</span>
              <p className="text-xs text-zinc-300 mt-0.5 whitespace-pre-wrap">{agent.summary}</p>
            </div>
          )}
          <div className="text-[10px] text-zinc-600 font-mono">
            {agent.sessionKey}
          </div>
        </div>
      )}
    </div>
  )
}
