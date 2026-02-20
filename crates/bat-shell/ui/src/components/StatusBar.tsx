import type { AgentStatus, SessionMeta } from '../types'

interface Props {
  session: SessionMeta | null
  agentStatus: AgentStatus
  error: string | null
}

const statusLabel: Record<AgentStatus, string> = {
  idle: 'Ready',
  thinking: 'Thinking\u2026',
  tool_calling: 'Using tool\u2026',
}

const statusColor: Record<AgentStatus, string> = {
  idle: 'text-emerald-400',
  thinking: 'text-yellow-400',
  tool_calling: 'text-indigo-400',
}

export function StatusBar({ session, agentStatus, error }: Props) {
  return (
    <div className="flex items-center gap-4 border-t border-zinc-700 bg-zinc-900 px-4 py-1.5 text-xs text-zinc-500">
      {session && (
        <>
          <span className="font-mono">{session.model.replace('anthropic/', '')}</span>
          <span>\u2191 {session.token_input.toLocaleString()} \u2193 {session.token_output.toLocaleString()} tokens</span>
        </>
      )}
      <span className={`ml-auto font-medium ${statusColor[agentStatus]}`}>
        {error ? <span className="text-red-400">Error: {error}</span> : statusLabel[agentStatus]}
      </span>
    </div>
  )
}
