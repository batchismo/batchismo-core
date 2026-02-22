import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import type { AuditEntry, AuditLevel, AuditCategory, BatEvent } from '../types'
import { getAuditLogs } from '../lib/tauri'

const LEVEL_COLORS: Record<AuditLevel, string> = {
  debug: 'text-zinc-500 bg-zinc-800',
  info: 'text-blue-400 bg-blue-900/30',
  warn: 'text-amber-400 bg-amber-900/30',
  error: 'text-red-400 bg-red-900/30',
}

const CATEGORY_COLORS: Record<AuditCategory, string> = {
  agent: 'text-purple-400',
  tool: 'text-emerald-400',
  gateway: 'text-cyan-400',
  ipc: 'text-orange-400',
  config: 'text-pink-400',
}

export function LogsPanel() {
  const [entries, setEntries] = useState<AuditEntry[]>([])
  const [levelFilter, setLevelFilter] = useState<AuditLevel | ''>('')
  const [categoryFilter, setCategoryFilter] = useState<AuditCategory | ''>('')
  const [search, setSearch] = useState('')
  const [pinToBottom, setPinToBottom] = useState(true)
  const [expandedId, setExpandedId] = useState<number | null>(null)
  const listRef = useRef<HTMLDivElement>(null)

  // Initial load
  useEffect(() => {
    loadLogs()
  }, [levelFilter, categoryFilter, search])

  // Listen for real-time audit events
  useEffect(() => {
    const unlisten = listen<BatEvent>('bat-event', (e) => {
      const payload = e.payload
      if (payload.type === 'AuditLog') {
        const newEntry: AuditEntry = {
          id: Date.now(), // temp id for real-time entries
          ts: new Date().toISOString(),
          sessionId: null,
          level: payload.level as AuditLevel,
          category: payload.category as AuditCategory,
          event: payload.event,
          summary: payload.summary,
          detailJson: payload.detail_json,
        }
        setEntries(prev => [...prev, newEntry])
      }
    })
    return () => { unlisten.then(f => f()) }
  }, [])

  // Auto-scroll
  useEffect(() => {
    if (pinToBottom && listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight
    }
  }, [entries, pinToBottom])

  async function loadLogs() {
    try {
      const logs = await getAuditLogs({
        level: levelFilter || null,
        category: categoryFilter || null,
        search: search || null,
        limit: 500,
      })
      // API returns newest first, reverse for chronological display
      setEntries(logs.reverse())
    } catch (e) {
      console.error('Failed to load audit logs:', e)
    }
  }

  function formatTime(ts: string): string {
    try {
      const d = new Date(ts)
      return d.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
    } catch {
      return ts.slice(11, 19)
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* Filters */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-zinc-800 bg-zinc-900/50 flex-shrink-0">
        <select
          value={levelFilter}
          onChange={e => setLevelFilter(e.target.value as AuditLevel | '')}
          className="bg-zinc-800 text-zinc-300 text-xs rounded px-2 py-1 border border-zinc-700 focus:outline-none focus:border-zinc-500"
        >
          <option value="">All Levels</option>
          <option value="debug">Debug</option>
          <option value="info">Info</option>
          <option value="warn">Warn</option>
          <option value="error">Error</option>
        </select>

        <select
          value={categoryFilter}
          onChange={e => setCategoryFilter(e.target.value as AuditCategory | '')}
          className="bg-zinc-800 text-zinc-300 text-xs rounded px-2 py-1 border border-zinc-700 focus:outline-none focus:border-zinc-500"
        >
          <option value="">All Categories</option>
          <option value="agent">Agent</option>
          <option value="tool">Tool</option>
          <option value="gateway">Gateway</option>
          <option value="ipc">IPC</option>
          <option value="config">Config</option>
        </select>

        <input
          type="text"
          placeholder="Search..."
          value={search}
          onChange={e => setSearch(e.target.value)}
          className="bg-zinc-800 text-zinc-300 text-xs rounded px-2 py-1 border border-zinc-700 focus:outline-none focus:border-zinc-500 flex-1 max-w-xs"
        />

        <div className="flex-1" />

        <button
          onClick={() => setPinToBottom(!pinToBottom)}
          title={pinToBottom ? 'Unpin from bottom' : 'Pin to bottom'}
          className={`text-xs px-2 py-1 rounded ${pinToBottom ? 'bg-indigo-600/30 text-indigo-400' : 'bg-zinc-800 text-zinc-500'}`}
        >
          {pinToBottom ? '📌 Auto-scroll' : '📌 Paused'}
        </button>

        <span className="text-xs text-zinc-500 tabular-nums">
          {entries.length} entries
        </span>
      </div>

      {/* Log list */}
      <div ref={listRef} className="flex-1 overflow-y-auto font-mono text-xs">
        {entries.length === 0 ? (
          <div className="flex items-center justify-center h-full text-zinc-600">
            No audit events yet. Send a message to generate some.
          </div>
        ) : (
          <table className="w-full">
            <tbody>
              {entries.map(entry => (
                <tr
                  key={entry.id}
                  onClick={() => setExpandedId(expandedId === entry.id ? null : entry.id)}
                  className="hover:bg-zinc-800/50 cursor-pointer border-b border-zinc-900"
                >
                  <td className="px-2 py-1 text-zinc-600 whitespace-nowrap w-20">
                    {formatTime(entry.ts)}
                  </td>
                  <td className="px-1 py-1 w-14">
                    <span className={`inline-block px-1.5 py-0.5 rounded text-[10px] font-medium uppercase ${LEVEL_COLORS[entry.level]}`}>
                      {entry.level}
                    </span>
                  </td>
                  <td className="px-1 py-1 w-16">
                    <span className={`text-[10px] font-medium uppercase ${CATEGORY_COLORS[entry.category]}`}>
                      {entry.category}
                    </span>
                  </td>
                  <td className="px-2 py-1 text-zinc-300 truncate max-w-0">
                    {entry.summary}
                    {expandedId === entry.id && entry.detailJson && (
                      <pre className="mt-1 p-2 bg-zinc-900 rounded text-zinc-500 text-[10px] whitespace-pre-wrap break-all">
                        {(() => {
                          try { return JSON.stringify(JSON.parse(entry.detailJson), null, 2) }
                          catch { return entry.detailJson }
                        })()}
                      </pre>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
