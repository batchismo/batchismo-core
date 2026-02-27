import { useState, useEffect } from 'react'
import type { ToolInfo } from '../../types'
import { getTools, toggleTool } from '../../lib/tauri'

export function ToolsPage() {
  const [tools, setTools] = useState<ToolInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [toggling, setToggling] = useState<string | null>(null)

  const load = async () => {
    try {
      const data = await getTools()
      setTools(data)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  const handleToggle = async (name: string, currentlyEnabled: boolean) => {
    setToggling(name)
    setError(null)
    try {
      await toggleTool(name, !currentlyEnabled)
      setTools(prev => prev.map(t =>
        t.name === name ? { ...t, enabled: !currentlyEnabled } : t
      ))
    } catch (e) {
      setError(String(e))
    } finally {
      setToggling(null)
    }
  }

  const toolIcon = (name: string) => {
    if (name === 'fs.read') return (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
    )
    if (name === 'fs.write') return (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
      </svg>
    )
    return (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" />
      </svg>
    )
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-white">Tools</h2>
        <p className="text-sm text-zinc-400 mt-1">
          Enable or disable tools the agent can use. Disabled tools are excluded from the next agent turn.
        </p>
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-700 rounded-lg px-4 py-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      {loading ? (
        <div className="text-zinc-500 text-sm text-center py-8">Loading…</div>
      ) : (
        <div className="space-y-3">
          {tools.map(tool => (
            <div
              key={tool.name}
              className={`flex items-center gap-4 border rounded-lg px-4 py-4 transition-colors ${
                tool.enabled
                  ? 'bg-zinc-800/50 border-zinc-700'
                  : 'bg-zinc-900/30 border-zinc-800 opacity-60'
              }`}
            >
              <div className={`flex-shrink-0 ${tool.enabled ? 'text-[#39FF14]' : 'text-zinc-600'}`}>
                {toolIcon(tool.name)}
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-white">
                  {tool.icon} {tool.displayName}
                </p>
                <p className="text-xs text-zinc-400 mt-0.5">{tool.description}</p>
                <p className="text-xs text-zinc-600 font-mono mt-0.5">{tool.name}</p>
              </div>
              <button
                onClick={() => handleToggle(tool.name, tool.enabled)}
                disabled={toggling === tool.name}
                className={`relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed ${
                  tool.enabled ? 'bg-[#39FF14]' : 'bg-zinc-600'
                }`}
                role="switch"
                aria-checked={tool.enabled}
                title={tool.enabled ? 'Disable tool' : 'Enable tool'}
              >
                <span
                  className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                    tool.enabled ? 'translate-x-5' : 'translate-x-0'
                  }`}
                />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
