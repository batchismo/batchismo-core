import { useEffect, useState } from 'react'
import type { SessionMeta } from '../types'
import { listSessions, createSession, switchSession, deleteSessionByKey, getActiveSessionKey } from '../lib/tauri'

interface Props {
  onSessionChange: (session: SessionMeta) => void
}

export function SessionSwitcher({ onSessionChange }: Props) {
  const [sessions, setSessions] = useState<SessionMeta[]>([])
  const [activeKey, setActiveKey] = useState('main')
  const [showNew, setShowNew] = useState(false)
  const [newName, setNewName] = useState('')
  const [expanded, setExpanded] = useState(false)

  const refresh = async () => {
    const [list, key] = await Promise.all([listSessions(), getActiveSessionKey()])
    setSessions(list)
    setActiveKey(key)
  }

  useEffect(() => { refresh() }, [])

  const handleSwitch = async (key: string) => {
    const session = await switchSession(key)
    setActiveKey(key)
    setExpanded(false)
    onSessionChange(session)
  }

  const handleCreate = async () => {
    if (!newName.trim()) return
    const session = await createSession(newName.trim())
    setNewName('')
    setShowNew(false)
    setActiveKey(session.key)
    onSessionChange(session)
    refresh()
  }

  const handleDelete = async (key: string) => {
    if (key === 'main') return
    await deleteSessionByKey(key)
    if (activeKey === key) {
      const session = await switchSession('main')
      setActiveKey('main')
      onSessionChange(session)
    }
    refresh()
  }

  return (
    <div className="relative">
      {/* Current session button */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-1.5 px-2 py-1 rounded-md text-xs
                   bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors border border-zinc-700"
      >
        <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
        <span className="font-medium truncate max-w-[120px]">{activeKey}</span>
        <svg className={`w-3 h-3 transition-transform ${expanded ? 'rotate-180' : ''}`}
          fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {/* Dropdown */}
      {expanded && (
        <div className="absolute top-full left-0 mt-1 w-56 bg-zinc-900 border border-zinc-700
                        rounded-lg shadow-xl z-50 overflow-hidden">
          <div className="p-1.5 space-y-0.5 max-h-64 overflow-y-auto">
            {sessions.map(s => (
              <div
                key={s.key}
                className={`flex items-center gap-2 px-2 py-1.5 rounded text-xs cursor-pointer
                  ${s.key === activeKey ? 'bg-zinc-700 text-white' : 'text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200'}`}
                onClick={() => handleSwitch(s.key)}
              >
                <span className={`w-1.5 h-1.5 rounded-full ${s.key === activeKey ? 'bg-emerald-400' : 'bg-zinc-600'}`} />
                <span className="flex-1 truncate font-medium">{s.key}</span>
                <span className="text-[10px] text-zinc-600">
                  {s.token_input + s.token_output > 0 ? `${s.token_input + s.token_output}t` : ''}
                </span>
                {s.key !== 'main' && (
                  <button
                    onClick={(e) => { e.stopPropagation(); handleDelete(s.key) }}
                    className="text-zinc-600 hover:text-red-400 transition-colors"
                    title="Delete session"
                  >
                    ×
                  </button>
                )}
              </div>
            ))}
          </div>

          <div className="border-t border-zinc-700 p-1.5">
            {showNew ? (
              <div className="flex gap-1">
                <input
                  autoFocus
                  value={newName}
                  onChange={e => setNewName(e.target.value)}
                  onKeyDown={e => { if (e.key === 'Enter') handleCreate(); if (e.key === 'Escape') setShowNew(false) }}
                  placeholder="Session name..."
                  className="flex-1 bg-zinc-800 border border-zinc-600 rounded px-2 py-1 text-xs text-white
                             placeholder-zinc-500 outline-none focus:border-[#39FF14]"
                />
                <button
                  onClick={handleCreate}
                  className="px-2 py-1 bg-[#39FF14] hover:bg-[#2bcc10] rounded text-xs text-black"
                >
                  +
                </button>
              </div>
            ) : (
              <button
                onClick={() => setShowNew(true)}
                className="w-full text-left px-2 py-1.5 rounded text-xs text-zinc-500 hover:text-zinc-300
                           hover:bg-zinc-800 transition-colors"
              >
                + New Session
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
