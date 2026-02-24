import { useState, useEffect } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import type { PathPolicy } from '../../types'
import { getPathPolicies, addPathPolicy, deletePathPolicy } from '../../lib/tauri'

const ACCESS_OPTIONS = [
  { value: 'read-only', label: 'Read Only' },
  { value: 'read-write', label: 'Read & Write' },
  { value: 'write-only', label: 'Write Only' },
]

export function PathPoliciesPage() {
  const [policies, setPolicies] = useState<PathPolicy[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Form state
  const [newPath, setNewPath] = useState('')
  const [newAccess, setNewAccess] = useState<'read-only' | 'read-write' | 'write-only'>('read-write')
  const [newRecursive, setNewRecursive] = useState(true)
  const [adding, setAdding] = useState(false)

  const handleBrowse = async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Select a folder to grant access' })
      if (selected) {
        setNewPath(selected as string)
      }
    } catch (e) {
      setError(String(e))
    }
  }

  const load = async () => {
    try {
      const data = await getPathPolicies()
      setPolicies(data)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!newPath.trim()) return
    setAdding(true)
    setError(null)
    try {
      await addPathPolicy(newPath.trim(), newAccess, newRecursive)
      setNewPath('')
      await load()
    } catch (e) {
      setError(String(e))
    } finally {
      setAdding(false)
    }
  }

  const handleDelete = async (id: number | undefined) => {
    if (id === undefined) return
    setError(null)
    try {
      await deletePathPolicy(id)
      await load()
    } catch (e) {
      setError(String(e))
    }
  }

  const accessBadge = (access: string) => {
    const colors: Record<string, string> = {
      'read-only': 'bg-blue-900/50 text-blue-300 border border-blue-700',
      'read-write': 'bg-green-900/50 text-green-300 border border-green-700',
      'write-only': 'bg-orange-900/50 text-orange-300 border border-orange-700',
    }
    return colors[access] ?? 'bg-zinc-800 text-zinc-300'
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-white">Path Policies</h2>
        <p className="text-sm text-zinc-400 mt-1">
          Control which directories the agent can access. Changes take effect on the next message.
        </p>
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-700 rounded-lg px-4 py-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      {/* Add new policy form */}
      <form onSubmit={handleAdd} className="bg-zinc-800/50 border border-zinc-700 rounded-lg p-4 space-y-3">
        <h3 className="text-sm font-medium text-zinc-300">Add New Policy</h3>
        <div className="flex gap-2">
          <input
            type="text"
            value={newPath}
            onChange={e => setNewPath(e.target.value)}
            placeholder="C:\Users\you\Documents"
            className="flex-1 bg-zinc-900 border border-zinc-600 rounded-md px-3 py-2 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-400"
          />
          <button
            type="button"
            onClick={handleBrowse}
            className="bg-zinc-700 hover:bg-zinc-600 text-white text-sm px-3 py-2 rounded-md transition-colors flex items-center gap-1.5 flex-shrink-0"
            title="Browse for folder"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
            Browse
          </button>
          <select
            value={newAccess}
            onChange={e => setNewAccess(e.target.value as typeof newAccess)}
            className="bg-zinc-900 border border-zinc-600 rounded-md px-3 py-2 text-sm text-white focus:outline-none focus:border-zinc-400"
          >
            {ACCESS_OPTIONS.map(o => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        </div>
        <div className="flex items-center justify-between">
          <label className="flex items-center gap-2 text-sm text-zinc-400 cursor-pointer">
            <input
              type="checkbox"
              checked={newRecursive}
              onChange={e => setNewRecursive(e.target.checked)}
              className="rounded border-zinc-600 bg-zinc-900 text-[#39FF14]"
            />
            Include subdirectories (recursive)
          </label>
          <button
            type="submit"
            disabled={adding || !newPath.trim()}
            className="bg-[#39FF14] hover:bg-[#2bcc10] disabled:opacity-40 disabled:cursor-not-allowed text-black text-sm px-4 py-2 rounded-md transition-colors"
          >
            {adding ? 'Adding…' : 'Add Policy'}
          </button>
        </div>
      </form>

      {/* Policy list */}
      <div className="space-y-2">
        {loading ? (
          <div className="text-zinc-500 text-sm text-center py-6">Loading…</div>
        ) : policies.length === 0 ? (
          <div className="text-zinc-500 text-sm text-center py-8 border border-dashed border-zinc-700 rounded-lg">
            No path policies configured. Add one above to allow file access.
          </div>
        ) : (
          policies.map((p, i) => (
            <div
              key={i}
              className="flex items-center gap-3 bg-zinc-800/50 border border-zinc-700 rounded-lg px-4 py-3"
            >
              <div className="flex-1 min-w-0">
                <p className="text-sm text-white font-mono truncate">{p.path}</p>
                {p.description && (
                  <p className="text-xs text-zinc-500 mt-0.5">{p.description}</p>
                )}
              </div>
              <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${accessBadge(p.access)}`}>
                {p.access}
              </span>
              <span className="text-xs text-zinc-500">
                {p.recursive ? 'recursive' : 'top-level'}
              </span>
              <button
                onClick={() => handleDelete(p.id)}
                className="text-zinc-500 hover:text-red-400 transition-colors p-1 rounded"
                title="Delete policy"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
