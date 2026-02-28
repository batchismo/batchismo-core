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
  // Try exact match first
  const pricing = MODEL_PRICING[model]
  if (pricing) {
    return (input / 1_000_000) * pricing.input + (output / 1_000_000) * pricing.output
  }
  // Fuzzy match by model family
  const m = model.toLowerCase()
  if (m.includes('opus')) return (input / 1_000_000) * 15 + (output / 1_000_000) * 75
  if (m.includes('sonnet')) return (input / 1_000_000) * 3 + (output / 1_000_000) * 15
  if (m.includes('haiku')) return (input / 1_000_000) * 0.25 + (output / 1_000_000) * 1.25
  return null
}

function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`
  if (usd < 1) return `$${usd.toFixed(3)}`
  return `$${usd.toFixed(2)}`
}

/** Color palette for model cards/bars */
const MODEL_COLORS = [
  'emerald', 'blue', 'purple', 'amber', 'rose', 'cyan', 'orange', 'indigo',
]

function getModelColor(index: number): string {
  return MODEL_COLORS[index % MODEL_COLORS.length]
}

/** Tailwind color classes by name */
const COLOR_CLASSES: Record<string, { text: string; bg: string; bgMuted: string; border: string }> = {
  emerald: { text: 'text-emerald-400', bg: 'bg-emerald-500', bgMuted: 'bg-emerald-500/20', border: 'border-emerald-500/30' },
  blue:    { text: 'text-blue-400',    bg: 'bg-blue-500',    bgMuted: 'bg-blue-500/20',    border: 'border-blue-500/30' },
  purple:  { text: 'text-purple-400',  bg: 'bg-purple-500',  bgMuted: 'bg-purple-500/20',  border: 'border-purple-500/30' },
  amber:   { text: 'text-amber-400',   bg: 'bg-amber-500',   bgMuted: 'bg-amber-500/20',   border: 'border-amber-500/30' },
  rose:    { text: 'text-rose-400',    bg: 'bg-rose-500',    bgMuted: 'bg-rose-500/20',    border: 'border-rose-500/30' },
  cyan:    { text: 'text-cyan-400',    bg: 'bg-cyan-500',    bgMuted: 'bg-cyan-500/20',    border: 'border-cyan-500/30' },
  orange:  { text: 'text-orange-400',  bg: 'bg-orange-500',  bgMuted: 'bg-orange-500/20',  border: 'border-orange-500/30' },
  indigo:  { text: 'text-indigo-400',  bg: 'bg-indigo-500',  bgMuted: 'bg-indigo-500/20',  border: 'border-indigo-500/30' },
}

/** Friendly model display name */
function displayModelName(model: string): string {
  // Strip provider prefix
  const name = model.replace(/^[^/]+\//, '')
  // Shorten known models
  if (name.includes('opus')) return 'Claude Opus'
  if (name.includes('sonnet-4')) return 'Claude Sonnet 4'
  if (name.includes('sonnet')) return 'Claude Sonnet 3.5'
  if (name.includes('haiku')) return 'Claude Haiku'
  return name
}

function ModelCard({ m, colorName }: { m: ModelUsage; colorName: string }) {
  const cost = estimateCost(m.model, m.tokenInput, m.tokenOutput)
  const totalTokens = m.tokenInput + m.tokenOutput
  const colors = COLOR_CLASSES[colorName] || COLOR_CLASSES.emerald

  return (
    <div className={`bg-zinc-900 border border-zinc-800 rounded-lg p-4 ${colors.border} border-l-2`}>
      <div className="flex items-start justify-between mb-3">
        <div>
          <div className={`text-sm font-semibold ${colors.text}`}>{displayModelName(m.model)}</div>
          <div className="text-[10px] text-zinc-600 font-mono mt-0.5">{m.model}</div>
        </div>
        {cost !== null && (
          <div className={`text-lg font-bold ${colors.text}`}>{formatCost(cost)}</div>
        )}
      </div>

      <div className="grid grid-cols-3 gap-2">
        <div>
          <div className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Input</div>
          <div className="text-sm font-medium text-zinc-300">{formatNum(m.tokenInput)}</div>
        </div>
        <div>
          <div className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Output</div>
          <div className="text-sm font-medium text-zinc-300">{formatNum(m.tokenOutput)}</div>
        </div>
        <div>
          <div className="text-[10px] uppercase tracking-wider text-zinc-500 font-semibold">Sessions</div>
          <div className="text-sm font-medium text-zinc-300">{m.sessionCount}</div>
        </div>
      </div>

      {/* Token distribution bar */}
      {totalTokens > 0 && (
        <div className="mt-3">
          <div className="flex h-1.5 rounded-full overflow-hidden bg-zinc-800">
            <div
              className={`${colors.bg} opacity-60`}
              style={{ width: `${(m.tokenInput / totalTokens) * 100}%` }}
              title={`Input: ${formatNum(m.tokenInput)}`}
            />
            <div
              className={colors.bg}
              style={{ width: `${(m.tokenOutput / totalTokens) * 100}%` }}
              title={`Output: ${formatNum(m.tokenOutput)}`}
            />
          </div>
          <div className="flex justify-between mt-1">
            <span className="text-[9px] text-zinc-600">in {((m.tokenInput / totalTokens) * 100).toFixed(0)}%</span>
            <span className="text-[9px] text-zinc-600">out {((m.tokenOutput / totalTokens) * 100).toFixed(0)}%</span>
          </div>
        </div>
      )}
    </div>
  )
}

function CostBreakdownBar({ models }: { models: { model: string; cost: number; colorName: string }[] }) {
  const totalCost = models.reduce((sum, m) => sum + m.cost, 0)
  if (totalCost === 0) return null

  return (
    <div>
      <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">Cost Breakdown</h3>
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
        {/* Stacked bar */}
        <div className="flex h-3 rounded-full overflow-hidden bg-zinc-800 mb-3">
          {models.map((m) => {
            const pct = (m.cost / totalCost) * 100
            const colors = COLOR_CLASSES[m.colorName] || COLOR_CLASSES.emerald
            return (
              <div
                key={m.model}
                className={colors.bg}
                style={{ width: `${pct}%` }}
                title={`${displayModelName(m.model)}: ${formatCost(m.cost)} (${pct.toFixed(0)}%)`}
              />
            )
          })}
        </div>
        {/* Legend */}
        <div className="flex flex-wrap gap-x-4 gap-y-1">
          {models.map((m) => {
            const colors = COLOR_CLASSES[m.colorName] || COLOR_CLASSES.emerald
            const pct = ((m.cost / totalCost) * 100).toFixed(0)
            return (
              <div key={m.model} className="flex items-center gap-1.5">
                <div className={`w-2 h-2 rounded-full ${colors.bg}`} />
                <span className="text-[10px] text-zinc-400">
                  {displayModelName(m.model)} · {formatCost(m.cost)} ({pct}%)
                </span>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
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

  // Build model cost data for breakdown bar
  const modelCosts = stats.byModel
    .map((m, i) => ({
      model: m.model,
      cost: estimateCost(m.model, m.tokenInput, m.tokenOutput) ?? 0,
      colorName: getModelColor(i),
    }))
    .sort((a, b) => b.cost - a.cost)

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

      {/* Cost Breakdown Bar */}
      {modelCosts.length > 1 && <CostBreakdownBar models={modelCosts} />}

      {/* Per-Model Cards */}
      {stats.byModel.length > 0 && (
        <div>
          <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">Usage by Model</h3>
          <div className="space-y-3">
            {stats.byModel
              .map((m, i) => ({ m, colorName: getModelColor(i) }))
              .sort((a, b) => {
                const costA = estimateCost(a.m.model, a.m.tokenInput, a.m.tokenOutput) ?? 0
                const costB = estimateCost(b.m.model, b.m.tokenInput, b.m.tokenOutput) ?? 0
                return costB - costA
              })
              .map(({ m, colorName }) => (
                <ModelCard key={m.model} m={m} colorName={colorName} />
              ))}
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
