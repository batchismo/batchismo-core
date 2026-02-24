import { useEffect, useState } from 'react'
import type { UsageStats } from '../types'
import { getUsageStats } from '../lib/tauri'

function formatNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toString()
}

export function UsagePanel() {
  const [stats, setStats] = useState<UsageStats | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    getUsageStats()
      .then(setStats)
      .catch(console.error)
      .finally(() => setLoading(false))
  }, [])

  if (loading) {
    return <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">Loading usage data...</div>
  }

  if (!stats) {
    return <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">Failed to load usage data</div>
  }

  const totalTokens = stats.totalInput + stats.totalOutput

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-6">
      {/* Summary Cards */}
      <div className="grid grid-cols-3 gap-3">
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
          <div className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Total Tokens</div>
          <div className="text-2xl font-bold text-white mt-1">{formatNum(totalTokens)}</div>
          <div className="text-xs text-zinc-500 mt-0.5">
            {formatNum(stats.totalInput)} in / {formatNum(stats.totalOutput)} out
          </div>
        </div>
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
          <div className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Est. Cost</div>
          <div className="text-2xl font-bold text-emerald-400 mt-1">${stats.estimatedCostUsd.toFixed(4)}</div>
          <div className="text-xs text-zinc-500 mt-0.5">Based on Anthropic pricing</div>
        </div>
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
          <div className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Sessions</div>
          <div className="text-2xl font-bold text-white mt-1">{stats.sessions.length}</div>
          <div className="text-xs text-zinc-500 mt-0.5">{stats.byModel.length} model(s) used</div>
        </div>
      </div>

      {/* By Model */}
      <div>
        <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">Usage by Model</h3>
        <div className="space-y-2">
          {stats.byModel.map(m => {
            const pct = totalTokens > 0 ? ((m.tokenInput + m.tokenOutput) / totalTokens * 100) : 0
            return (
              <div key={m.model} className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-sm font-medium text-zinc-200">{m.model}</span>
                  <span className="text-xs text-zinc-500">{m.sessionCount} session(s)</span>
                </div>
                <div className="w-full bg-zinc-800 rounded-full h-1.5 mb-1.5">
                  <div
                    className="bg-indigo-500 h-1.5 rounded-full transition-all"
                    style={{ width: `${Math.max(pct, 1)}%` }}
                  />
                </div>
                <div className="flex justify-between text-xs text-zinc-500">
                  <span>{formatNum(m.tokenInput)} in / {formatNum(m.tokenOutput)} out</span>
                  <span>{pct.toFixed(1)}%</span>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* By Session */}
      <div>
        <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">Usage by Session</h3>
        <div className="border border-zinc-800 rounded-lg overflow-hidden">
          <table className="w-full text-xs">
            <thead>
              <tr className="bg-zinc-900 text-zinc-500 border-b border-zinc-800">
                <th className="text-left px-3 py-2 font-semibold">Session</th>
                <th className="text-left px-3 py-2 font-semibold">Model</th>
                <th className="text-right px-3 py-2 font-semibold">Input</th>
                <th className="text-right px-3 py-2 font-semibold">Output</th>
                <th className="text-right px-3 py-2 font-semibold">Messages</th>
              </tr>
            </thead>
            <tbody>
              {stats.sessions.map(s => (
                <tr key={s.key} className="border-b border-zinc-800/50 hover:bg-zinc-900/50">
                  <td className="px-3 py-2 text-zinc-200 font-medium">{s.key}</td>
                  <td className="px-3 py-2 text-zinc-500">{s.model}</td>
                  <td className="px-3 py-2 text-right text-zinc-400">{formatNum(s.tokenInput)}</td>
                  <td className="px-3 py-2 text-right text-zinc-400">{formatNum(s.tokenOutput)}</td>
                  <td className="px-3 py-2 text-right text-zinc-500">{s.messageCount}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
