import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { AuditStats, AuditEntry, AuditFilter } from '../../types'

function StatCard({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div className="bg-zinc-800 border border-zinc-700 rounded-lg p-4 text-center">
      <div className={`text-2xl font-bold ${color}`}>{value.toLocaleString()}</div>
      <div className="text-xs text-zinc-400 mt-1">{label}</div>
    </div>
  )
}

function BarSegment({ label, value, total, color }: { label: string; value: number; total: number; color: string }) {
  const pct = total > 0 ? (value / total) * 100 : 0
  return (
    <div className="flex items-center gap-3 text-sm">
      <span className="w-16 text-zinc-400 text-right">{label}</span>
      <div className="flex-1 bg-zinc-800 rounded-full h-4 overflow-hidden">
        <div className={`h-full rounded-full ${color}`} style={{ width: `${Math.max(pct, 1)}%` }} />
      </div>
      <span className="w-12 text-zinc-300 text-right">{value}</span>
    </div>
  )
}

export function AuditPage() {
  const [stats, setStats] = useState<AuditStats | null>(null)
  const [recent, setRecent] = useState<AuditEntry[]>([])
  const [error, setError] = useState('')

  const refresh = async () => {
    try {
      const s = await invoke<AuditStats>('get_audit_stats')
      setStats(s)
      const filter: AuditFilter = { limit: 20 }
      const entries = await invoke<AuditEntry[]>('get_audit_logs', { filter })
      setRecent(entries)
      setError('')
    } catch (e) {
      setError(String(e))
    }
  }

  useEffect(() => { refresh() }, [])

  if (error) {
    return <div className="text-red-400 text-sm">{error}</div>
  }
  if (!stats) {
    return <div className="text-zinc-500 text-sm">Loading audit data…</div>
  }

  const levelColors: Record<string, string> = {
    debug: 'text-zinc-500',
    info: 'text-blue-400',
    warn: 'text-yellow-400',
    error: 'text-red-400',
  }

  return (
    <div className="max-w-2xl">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-lg font-semibold text-white">Audit Dashboard</h2>
          <p className="text-sm text-zinc-400">Security and activity overview.</p>
        </div>
        <button
          onClick={refresh}
          className="px-3 py-1.5 bg-zinc-700 hover:bg-zinc-600 text-zinc-300 text-sm rounded-md transition-colors"
        >
          Refresh
        </button>
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-4 gap-3 mb-6">
        <StatCard label="Total" value={stats.total} color="text-white" />
        <StatCard label="Info" value={stats.byLevel.info} color="text-blue-400" />
        <StatCard label="Warnings" value={stats.byLevel.warn} color="text-yellow-400" />
        <StatCard label="Errors" value={stats.byLevel.error} color="text-red-400" />
      </div>

      {/* By level */}
      <h3 className="text-sm font-medium text-zinc-300 mb-2">By Level</h3>
      <div className="space-y-2 mb-6">
        <BarSegment label="Debug" value={stats.byLevel.debug} total={stats.total} color="bg-zinc-600" />
        <BarSegment label="Info" value={stats.byLevel.info} total={stats.total} color="bg-blue-500" />
        <BarSegment label="Warn" value={stats.byLevel.warn} total={stats.total} color="bg-yellow-500" />
        <BarSegment label="Error" value={stats.byLevel.error} total={stats.total} color="bg-red-500" />
      </div>

      {/* By category */}
      <h3 className="text-sm font-medium text-zinc-300 mb-2">By Category</h3>
      <div className="space-y-2 mb-6">
        <BarSegment label="Agent" value={stats.byCategory.agent} total={stats.total} color="bg-purple-500" />
        <BarSegment label="Tool" value={stats.byCategory.tool} total={stats.total} color="bg-green-500" />
        <BarSegment label="Gateway" value={stats.byCategory.gateway} total={stats.total} color="bg-blue-500" />
        <BarSegment label="IPC" value={stats.byCategory.ipc} total={stats.total} color="bg-orange-500" />
        <BarSegment label="Config" value={stats.byCategory.config} total={stats.total} color="bg-cyan-500" />
      </div>

      {/* Recent entries */}
      <h3 className="text-sm font-medium text-zinc-300 mb-2">Recent Entries</h3>
      <div className="bg-zinc-800 border border-zinc-700 rounded-lg overflow-hidden">
        {recent.length === 0 ? (
          <p className="text-zinc-500 text-sm p-4">No audit entries yet.</p>
        ) : (
          <div className="divide-y divide-zinc-700/50 max-h-80 overflow-y-auto">
            {recent.map(entry => (
              <div key={entry.id} className="px-3 py-2 flex items-start gap-2 text-xs">
                <span className={`font-mono uppercase w-10 flex-shrink-0 ${levelColors[entry.level] || 'text-zinc-400'}`}>
                  {entry.level}
                </span>
                <span className="text-zinc-500 w-14 flex-shrink-0">{entry.category}</span>
                <span className="text-zinc-300 flex-1 break-words">{entry.summary}</span>
                <span className="text-zinc-600 flex-shrink-0 whitespace-nowrap">
                  {new Date(entry.ts).toLocaleTimeString()}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
