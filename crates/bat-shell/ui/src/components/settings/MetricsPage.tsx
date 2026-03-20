import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { UsageStats, AuditStats, SessionMeta, SubagentInfo } from '../../types'

interface MetricsData {
  usageStats: UsageStats
  auditStats: AuditStats
  totalSessions: number
  activeSessions: number
  runningSubagents: number
}

export function MetricsPage() {
  const [metrics, setMetrics] = useState<MetricsData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Load metrics on component mount and periodically refresh
  useEffect(() => {
    loadMetrics()
    const interval = setInterval(loadMetrics, 30000) // Refresh every 30 seconds
    return () => clearInterval(interval)
  }, [])

  const loadMetrics = async () => {
    try {
      setError(null)
      
      const [usageStats, auditStats, sessions, subagents] = await Promise.all([
        invoke<UsageStats>('get_usage_stats'),
        invoke<AuditStats>('get_audit_stats'),
        invoke<SessionMeta[]>('list_sessions'),
        invoke<SubagentInfo[]>('get_subagents'),
      ])

      const activeSessions = sessions.filter((s: SessionMeta) => s.status === 'active').length
      const runningSubagents = subagents.filter((s: SubagentInfo) => s.status === 'running').length

      setMetrics({
        usageStats,
        auditStats,
        totalSessions: sessions.length,
        activeSessions,
        runningSubagents,
      })
    } catch (err) {
      console.error('Failed to load metrics:', err)
      setError('Failed to load metrics')
    } finally {
      setLoading(false)
    }
  }

  const formatNumber = (num: number) => {
    return new Intl.NumberFormat().format(num)
  }

  const formatCost = (cost: number) => {
    return `$${cost.toFixed(4)}`
  }

  const formatDate = (date: string) => {
    try {
      return new Date(date).toLocaleDateString()
    } catch {
      return 'Unknown'
    }
  }

  const getRecentUsage = () => {
    if (!metrics) return { today: 0, thisWeek: 0 }
    
    const now = new Date()
    const today = now.toISOString().split('T')[0]
    const weekAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]

    // This is simplified - in a real implementation, we'd need actual daily/weekly breakdowns
    const recentSessions = metrics.usageStats.sessions.filter(s => {
      const lastActive = s.lastActive.split('T')[0]
      return lastActive >= weekAgo
    })

    const todaySessions = recentSessions.filter(s => s.lastActive.split('T')[0] === today)

    return {
      today: todaySessions.reduce((sum, s) => sum + s.tokenInput + s.tokenOutput, 0),
      thisWeek: recentSessions.reduce((sum, s) => sum + s.tokenInput + s.tokenOutput, 0),
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="bg-red-950 border border-red-800 rounded-md p-4">
        <div className="flex">
          <div className="ml-3">
            <h3 className="text-sm font-medium text-red-200">Error</h3>
            <div className="mt-2 text-sm text-red-300">{error}</div>
          </div>
        </div>
        <button
          onClick={loadMetrics}
          className="mt-3 bg-red-800 hover:bg-red-700 text-white px-3 py-1 rounded text-sm"
        >
          Retry
        </button>
      </div>
    )
  }

  if (!metrics) {
    return <div className="text-zinc-500">No metrics data available</div>
  }

  const recentUsage = getRecentUsage()

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-white mb-2">Metrics Dashboard</h2>
          <p className="text-zinc-400 text-sm">
            Overview of system usage, performance, and activity metrics.
          </p>
        </div>
        <button
          onClick={loadMetrics}
          className="text-zinc-400 hover:text-white p-2 rounded-md hover:bg-zinc-800 transition-colors"
          title="Refresh metrics"
        >
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        </button>
      </div>

      {/* Key Metrics Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-zinc-800 rounded-lg p-4 border border-zinc-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-zinc-400 text-sm">Total Sessions</p>
              <p className="text-2xl font-semibold text-white">{formatNumber(metrics.totalSessions)}</p>
            </div>
            <div className="w-10 h-10 bg-blue-500/20 rounded-lg flex items-center justify-center">
              <svg className="w-5 h-5 text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
              </svg>
            </div>
          </div>
          <p className="text-zinc-500 text-xs mt-2">{metrics.activeSessions} active</p>
        </div>

        <div className="bg-zinc-800 rounded-lg p-4 border border-zinc-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-zinc-400 text-sm">Token Usage Today</p>
              <p className="text-2xl font-semibold text-white">{formatNumber(recentUsage.today)}</p>
            </div>
            <div className="w-10 h-10 bg-green-500/20 rounded-lg flex items-center justify-center">
              <svg className="w-5 h-5 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
            </div>
          </div>
          <p className="text-zinc-500 text-xs mt-2">{formatNumber(recentUsage.thisWeek)} this week</p>
        </div>

        <div className="bg-zinc-800 rounded-lg p-4 border border-zinc-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-zinc-400 text-sm">Active Subagents</p>
              <p className="text-2xl font-semibold text-white">{formatNumber(metrics.runningSubagents)}</p>
            </div>
            <div className="w-10 h-10 bg-purple-500/20 rounded-lg flex items-center justify-center">
              <svg className="w-5 h-5 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            </div>
          </div>
          <p className="text-zinc-500 text-xs mt-2">Background tasks</p>
        </div>

        <div className="bg-zinc-800 rounded-lg p-4 border border-zinc-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-zinc-400 text-sm">Estimated Cost</p>
              <p className="text-2xl font-semibold text-white">{formatCost(metrics.usageStats.estimatedCostUsd)}</p>
            </div>
            <div className="w-10 h-10 bg-yellow-500/20 rounded-lg flex items-center justify-center">
              <svg className="w-5 h-5 text-yellow-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1" />
              </svg>
            </div>
          </div>
          <p className="text-zinc-500 text-xs mt-2">Total across all models</p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Model Usage */}
        <div className="bg-zinc-800 rounded-lg p-4 border border-zinc-700">
          <h3 className="text-lg font-medium text-white mb-4">Usage by Model</h3>
          <div className="space-y-3">
            {metrics.usageStats.byModel.length === 0 ? (
              <p className="text-zinc-500 text-sm">No model usage data available</p>
            ) : (
              metrics.usageStats.byModel.slice(0, 5).map((model) => (
                <div key={model.model} className="flex items-center justify-between">
                  <div>
                    <p className="text-white font-medium">{model.model}</p>
                    <p className="text-zinc-400 text-sm">{formatNumber(model.sessionCount)} sessions</p>
                  </div>
                  <div className="text-right">
                    <p className="text-white">{formatNumber(model.tokenInput + model.tokenOutput)}</p>
                    <p className="text-zinc-500 text-sm">tokens</p>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Recent Activity */}
        <div className="bg-zinc-800 rounded-lg p-4 border border-zinc-700">
          <h3 className="text-lg font-medium text-white mb-4">Recent Sessions</h3>
          <div className="space-y-3">
            {metrics.usageStats.sessions.length === 0 ? (
              <p className="text-zinc-500 text-sm">No session data available</p>
            ) : (
              metrics.usageStats.sessions
                .sort((a, b) => new Date(b.lastActive).getTime() - new Date(a.lastActive).getTime())
                .slice(0, 5)
                .map((session) => (
                  <div key={session.key} className="flex items-center justify-between">
                    <div>
                      <p className="text-white font-medium">{session.key}</p>
                      <p className="text-zinc-400 text-sm">{session.model}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-white">{formatNumber(session.tokenInput + session.tokenOutput)}</p>
                      <p className="text-zinc-500 text-sm">{formatDate(session.lastActive)}</p>
                    </div>
                  </div>
                ))
            )}
          </div>
        </div>
      </div>

      {/* System Activity */}
      <div className="bg-zinc-800 rounded-lg p-4 border border-zinc-700">
        <h3 className="text-lg font-medium text-white mb-4">System Activity</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="text-center">
            <p className="text-2xl font-semibold text-blue-400">{formatNumber(metrics.auditStats.total)}</p>
            <p className="text-zinc-400 text-sm">Total Events</p>
          </div>
          <div className="text-center">
            <p className="text-2xl font-semibold text-green-400">{formatNumber(metrics.auditStats.byLevel.info)}</p>
            <p className="text-zinc-400 text-sm">Info Events</p>
          </div>
          <div className="text-center">
            <p className="text-2xl font-semibold text-yellow-400">{formatNumber(metrics.auditStats.byLevel.warn)}</p>
            <p className="text-zinc-400 text-sm">Warnings</p>
          </div>
          <div className="text-center">
            <p className="text-2xl font-semibold text-red-400">{formatNumber(metrics.auditStats.byLevel.error)}</p>
            <p className="text-zinc-400 text-sm">Errors</p>
          </div>
        </div>
      </div>

      {/* Total Usage Summary */}
      <div className="bg-zinc-800 rounded-lg p-4 border border-zinc-700">
        <h3 className="text-lg font-medium text-white mb-4">Total Usage Summary</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <p className="text-zinc-400 text-sm">Input Tokens</p>
            <p className="text-xl font-semibold text-white">{formatNumber(metrics.usageStats.totalInput)}</p>
          </div>
          <div>
            <p className="text-zinc-400 text-sm">Output Tokens</p>
            <p className="text-xl font-semibold text-white">{formatNumber(metrics.usageStats.totalOutput)}</p>
          </div>
          <div>
            <p className="text-zinc-400 text-sm">Combined Total</p>
            <p className="text-xl font-semibold text-white">
              {formatNumber(metrics.usageStats.totalInput + metrics.usageStats.totalOutput)}
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}