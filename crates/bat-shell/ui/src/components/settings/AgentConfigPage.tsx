import { useState, useEffect } from 'react'
import type { BatConfig } from '../../types'
import { getConfig, updateConfig, getSystemPrompt } from '../../lib/tauri'

const COMMON_MODELS = [
  'anthropic/claude-opus-4-6',
  'anthropic/claude-sonnet-4-6',
  'anthropic/claude-haiku-4-5-20251001',
]

export function AgentConfigPage() {
  const [config, setConfig] = useState<BatConfig | null>(null)
  const [systemPrompt, setSystemPrompt] = useState<string>('')
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [loadingPrompt, setLoadingPrompt] = useState(false)
  const [showPrompt, setShowPrompt] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [saved, setSaved] = useState(false)
  const [showApiKey, setShowApiKey] = useState(false)

  useEffect(() => {
    getConfig()
      .then(setConfig)
      .catch(e => setError(String(e)))
      .finally(() => setLoading(false))
  }, [])

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!config) return
    setSaving(true)
    setError(null)
    setSaved(false)
    try {
      await updateConfig(config)
      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch (e) {
      setError(String(e))
    } finally {
      setSaving(false)
    }
  }

  const handleLoadPrompt = async () => {
    setLoadingPrompt(true)
    setError(null)
    try {
      const prompt = await getSystemPrompt()
      setSystemPrompt(prompt)
      setShowPrompt(true)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoadingPrompt(false)
    }
  }

  const updateAgent = (patch: Partial<BatConfig['agent']>) => {
    if (!config) return
    setConfig({ ...config, agent: { ...config.agent, ...patch } })
  }

  if (loading) {
    return <div className="text-zinc-500 text-sm text-center py-8">Loading…</div>
  }

  if (!config) {
    return <div className="text-red-400 text-sm">{error ?? 'Failed to load config.'}</div>
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-white">Agent Configuration</h2>
        <p className="text-sm text-zinc-400 mt-1">
          Configure the AI agent's identity and model settings.
        </p>
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-700 rounded-lg px-4 py-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      {saved && (
        <div className="bg-green-900/30 border border-green-700 rounded-lg px-4 py-3 text-green-300 text-sm">
          Settings saved successfully.
        </div>
      )}

      <form onSubmit={handleSave} className="space-y-5">
        {/* Agent Name */}
        <div className="space-y-1.5">
          <label className="block text-sm font-medium text-zinc-300">Agent Name</label>
          <input
            type="text"
            value={config.agent.name}
            onChange={e => updateAgent({ name: e.target.value })}
            className="w-full bg-zinc-900 border border-zinc-600 rounded-md px-3 py-2 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-400"
            placeholder="Aria"
          />
          <p className="text-xs text-zinc-500">The agent's name used in the system prompt.</p>
        </div>

        {/* Model */}
        <div className="space-y-1.5">
          <label className="block text-sm font-medium text-zinc-300">Model</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={config.agent.model}
              onChange={e => updateAgent({ model: e.target.value })}
              className="flex-1 bg-zinc-900 border border-zinc-600 rounded-md px-3 py-2 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-400 font-mono"
              placeholder="anthropic/claude-opus-4-6"
            />
          </div>
          <div className="flex flex-wrap gap-1.5 mt-1.5">
            {COMMON_MODELS.map(m => (
              <button
                key={m}
                type="button"
                onClick={() => updateAgent({ model: m })}
                className={`text-xs px-2 py-1 rounded border transition-colors font-mono ${
                  config.agent.model === m
                    ? 'border-indigo-500 bg-indigo-900/40 text-indigo-300'
                    : 'border-zinc-700 bg-zinc-800 text-zinc-400 hover:border-zinc-500 hover:text-zinc-300'
                }`}
              >
                {m.replace('anthropic/', '')}
              </button>
            ))}
          </div>
        </div>

        {/* API Key */}
        <div className="space-y-1.5">
          <label className="block text-sm font-medium text-zinc-300">API Key</label>
          <div className="relative">
            <input
              type={showApiKey ? 'text' : 'password'}
              value={config.agent.api_key ?? ''}
              onChange={e => updateAgent({ api_key: e.target.value || null })}
              className="w-full bg-zinc-900 border border-zinc-600 rounded-md px-3 py-2 pr-10 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-400 font-mono"
              placeholder="sk-ant-…  (or set ANTHROPIC_API_KEY env var)"
            />
            <button
              type="button"
              onClick={() => setShowApiKey(v => !v)}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
            >
              {showApiKey ? (
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                </svg>
              ) : (
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                </svg>
              )}
            </button>
          </div>
          <p className="text-xs text-zinc-500">
            Stored in config.toml. The ANTHROPIC_API_KEY environment variable takes priority if set.
          </p>
        </div>

        {/* Workspace dir (read-only info) */}
        <div className="space-y-1.5">
          <label className="block text-sm font-medium text-zinc-300">Workspace Directory</label>
          <div className="bg-zinc-900/50 border border-zinc-700 rounded-md px-3 py-2 text-sm text-zinc-400 font-mono">
            ~/.batchismo/workspace/
          </div>
          <p className="text-xs text-zinc-500">
            Contains IDENTITY.md, MEMORY.md, and SKILLS.md — editable in a text editor.
          </p>
        </div>

        <div className="flex gap-3">
          <button
            type="submit"
            disabled={saving}
            className="bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white text-sm px-5 py-2 rounded-md transition-colors"
          >
            {saving ? 'Saving…' : 'Save Changes'}
          </button>
        </div>
      </form>

      {/* System Prompt Preview */}
      <div className="border-t border-zinc-700 pt-5 space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-medium text-zinc-300">System Prompt Preview</h3>
            <p className="text-xs text-zinc-500 mt-0.5">The full prompt sent to the model on each turn.</p>
          </div>
          <button
            type="button"
            onClick={handleLoadPrompt}
            disabled={loadingPrompt}
            className="text-sm text-indigo-400 hover:text-indigo-300 disabled:opacity-40 transition-colors"
          >
            {loadingPrompt ? 'Loading…' : showPrompt ? 'Refresh' : 'Preview'}
          </button>
        </div>
        {showPrompt && (
          <pre className="bg-zinc-900 border border-zinc-700 rounded-lg p-4 text-xs text-zinc-300 overflow-auto max-h-80 whitespace-pre-wrap font-mono leading-relaxed">
            {systemPrompt}
          </pre>
        )}
      </div>
    </div>
  )
}
