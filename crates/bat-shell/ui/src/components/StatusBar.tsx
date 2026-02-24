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
  tool_calling: 'text-[#39FF14]',
}

// Claude context window limit
const CONTEXT_LIMIT = 200_000

function tokenColor(total: number): string {
  const ratio = total / CONTEXT_LIMIT
  if (ratio < 0.5) return 'text-emerald-400'
  if (ratio < 0.75) return 'text-yellow-400'
  if (ratio < 0.9) return 'text-orange-400'
  return 'text-red-400'
}

export function StatusBar({ session, agentStatus, error }: Props) {
  const totalTokens = session ? session.token_input + session.token_output : 0
  const color = tokenColor(totalTokens)

  return (
    <div className="flex items-center gap-4 border-t border-zinc-700 bg-zinc-900 px-4 py-1.5 text-xs text-zinc-500">
      {session && (
        <>
          <span className="font-mono">{session.model.replace('anthropic/', '')}</span>
          <span className={color}>
            in: {session.token_input.toLocaleString()} · out: {session.token_output.toLocaleString()} · total: {totalTokens.toLocaleString()} / {CONTEXT_LIMIT.toLocaleString()}
          </span>
        </>
      )}
      <span className={`ml-auto font-medium ${statusColor[agentStatus]}`}>
        {error ? <span className="text-red-400">Error: {error}</span> : statusLabel[agentStatus]}
      </span>
    </div>
  )
}
