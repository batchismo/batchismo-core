import { useEffect, useState } from 'react'
import type { UsageStats, SessionUsage, ModelUsage } from '../types'
import { getUsageStats } from '../lib/tauri'

function formatNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toString()
}

// Approximate pricing per million tokens (USD)
const MODEL_PRICING: Record<string, { input: number; output: number }> = {
  'claude-opus-4-6': { input: 15, output: 75 },
  'anthropic/claude-opus-4-6': { input: 15, output: 75 },
  'claude-sonnet-4-20250514': { input: 3, output: 15 },
  'anthropic/claude-sonnet-4-20250514': { input: 3, output: 15 },
  'claude-sonnet-4-0-20250514': { input: 3, output: 15 },
  'claude-3-5-sonnet-20241022': { input: 3, output: 15 },
  'anthropic/claude-3-5-sonnet-20241022': { input: 3, output: 15 },
  'claude-haiku-3-20240307': { input: 0.25, output: 1.25 },
  'anthropic/claude-haiku-3-20240307': { input: 0.25, output: 1.25 },
}

function estimateCost(model: string, input: number, output: number): number | null {
  const pricing = MODEL_PRICING[model]
  if (!pricing) return null
  return (input / 1_000_000) * pricing.input + (output / 1_000_000) * pricing.output
}

function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`
  if (usd < 1) return `$${usd.toFixed(3)}`
  return `$${usd.toFixed(2)}`
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
    <div className="flex-1 overflow-y-auto overflow-x-hidden p-4 space-y-6">
      {/* Hero Cost Card */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-5 text-center">
        <div className="text-[10px] uppercase tracking-widest text-zinc-500 font-semibold mb-1">Estimated Cost</div>
        <div className="text-4xl font-bold text-emerald-400">{formatCost(stats.estimatedCostUsd)}</div>
        <div className="text-xs text-zinc-500 mt-1">Based on Anthropic pricing · all time</div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-2 gap-3">
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
          <div className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Total Tokens</div>
          <div className="text-xl font-bold text-white mt-1">{formatNum(totalTokens)}</div>
          <div className="text-xs text-zinc-500 mt-0.5">
            {formatNum(stats.totalInput)} in · {formatNum(stats.totalOutput)} out
          </div>
        </div>
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
          <div className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Sessions</div>
          <div className="text-xl font-bold text-white mt-1">{stats.sessions.length}</div>
          <div className="text-xs text-zinc-500 mt-0.5">{stats.byModel.length} model(s) used</div>
        </div>
      </div>

      {/* By Model — simple text summary */}
      {stats.byModel.length > 0 && (
        <div>
          <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">Cost by Model</h3>
          <div className="bg-zinc-900 border border-zinc-800 rounded-lg divide-y divide-zinc-800">
            {stats.byModel.map((m: ModelUsage) => {
              const cost = estimateCost(m.model, m.tokenInput, m.tokenOutput)
              return (
                <div key={m.model} className="flex items-center justify-between px-3 py-2.5">
                  <div>
                    <span className="text-sm text-zinc-200">{m.model}</span>
                    <span className="text-xs text-zinc-600 ml-2">{m.sessionCount} session(s)</span>
                  </div>
                  <div className="text-right">
                    {cost !== null && (
                      <span className="text-sm font-medium text-emerald-400 mr-3">{formatCost(cost)}</span>
                    )}
                    <span className="text-xs text-zinc-500">
                      {formatNum(m.tokenInput + m.tokenOutput)} tokens
                    </span>
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      )}

      {/* By Session */}
      {stats.sessions.length > 0 && (
        <div>
          <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">Usage by Session</h3>
          <div className="border border-zinc-800 rounded-lg overflow-hidden overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="bg-zinc-900 text-zinc-500 border-b border-zinc-800">
                  <th className="text-left px-3 py-2 font-semibold">Session</th>
                  <th className="text-left px-3 py-2 font-semibold">Model</th>
                  <th className="text-right px-3 py-2 font-semibold">Tokens</th>
                  <th className="text-right px-3 py-2 font-semibold">Cost</th>
                  <th className="text-right px-3 py-2 font-semibold">Msgs</th>
                </tr>
              </thead>
              <tbody>
                {stats.sessions.map((s: SessionUsage) => {
                  const cost = estimateCost(s.model, s.tokenInput, s.tokenOutput)
                  return (
                    <tr key={s.key} className="border-b border-zinc-800/50 hover:bg-zinc-900/50">
                      <td className="px-3 py-2 text-zinc-200 font-medium">{s.key}</td>
                      <td className="px-3 py-2 text-zinc-500">{s.model}</td>
                      <td className="px-3 py-2 text-right text-zinc-400">
                        {formatNum(s.tokenInput + s.tokenOutput)}
                      </td>
                      <td className="px-3 py-2 text-right text-emerald-400 font-medium">
                        {cost !== null ? formatCost(cost) : '—'}
                      </td>
                      <td className="px-3 py-2 text-right text-zinc-500">{s.messageCount}</td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  )
}
