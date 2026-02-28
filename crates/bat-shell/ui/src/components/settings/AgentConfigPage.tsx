import { useState, useEffect } from 'react'
import type { BatConfig } from '../../types'
import { getConfig, updateConfig, getSystemPrompt } from '../../lib/tauri'

const ANTHROPIC_MODELS = [
  { id: 'claude-sonnet-4-6', label: 'Claude Sonnet 4.6', desc: 'Fast & capable', provider: 'Anthropic' },
  { id: 'claude-opus-4-6', label: 'Claude Opus 4.6', desc: 'Most powerful', provider: 'Anthropic' },
  { id: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5', desc: 'Fastest & cheapest', provider: 'Anthropic' },
]

const OPENAI_MODELS = [
  { id: 'gpt-4o', label: 'GPT-4o', desc: 'Flagship multimodal', provider: 'OpenAI' },
  { id: 'gpt-4o-mini', label: 'GPT-4o Mini', desc: 'Fast & affordable', provider: 'OpenAI' },
  { id: 'o3-mini', label: 'o3-mini', desc: 'Reasoning model', provider: 'OpenAI' },
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

  const hasAnthropicKey = !!(config?.api_keys?.anthropic)
  const hasOpenAIKey = !!(config?.api_keys?.openai)

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

  const toggleModel = (modelId: string) => {
    if (!config) return
    const enabled = config.agent.enabled_models ?? []
    // Default model must always stay enabled
    if (modelId === config.agent.model && enabled.includes(modelId)) return
    const next = enabled.includes(modelId)
      ? enabled.filter(id => id !== modelId)
      : [...enabled, modelId]
    updateAgent({ enabled_models: next })
  }

  const isModelEnabled = (modelId: string) => {
    const enabled = config?.agent.enabled_models ?? []
    return enabled.includes(modelId) || modelId === config?.agent.model
  }

  // All available models based on configured keys
  const availableModels = [
    ...(hasAnthropicKey ? ANTHROPIC_MODELS : []),
    ...(hasOpenAIKey ? OPENAI_MODELS : []),
  ]

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
          <label className="block text-sm font-medium text-zinc-300">Default Model</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={config.agent.model}
              onChange={e => updateAgent({ model: e.target.value })}
              className="flex-1 bg-zinc-900 border border-zinc-600 rounded-md px-3 py-2 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-400 font-mono"
              placeholder="claude-sonnet-4-6"
            />
          </div>

          {/* Anthropic models */}
          {hasAnthropicKey && (
            <div className="mt-2">
              <p className="text-[10px] text-zinc-500 uppercase tracking-wider mb-1.5 font-medium">Anthropic</p>
              <div className="flex flex-wrap gap-1.5">
                {ANTHROPIC_MODELS.map(m => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => {
                      const enabled = config.agent.enabled_models ?? []
                      updateAgent({
                        model: m.id,
                        enabled_models: enabled.includes(m.id) ? enabled : [...enabled, m.id],
                      })
                    }}
                    className={`text-xs px-2.5 py-1.5 rounded border transition-colors ${
                      config.agent.model === m.id
                        ? 'border-[#39FF14] bg-[#39FF14]/10 text-[#39FF14]'
                        : 'border-zinc-700 bg-zinc-800 text-zinc-400 hover:border-zinc-500 hover:text-zinc-300'
                    }`}
                  >
                    <span className="font-mono">{m.label}</span>
                    <span className="text-zinc-600 ml-1.5">· {m.desc}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* OpenAI models */}
          {hasOpenAIKey && (
            <div className="mt-2">
              <p className="text-[10px] text-zinc-500 uppercase tracking-wider mb-1.5 font-medium">OpenAI</p>
              <div className="flex flex-wrap gap-1.5">
                {OPENAI_MODELS.map(m => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => {
                      const enabled = config.agent.enabled_models ?? []
                      updateAgent({
                        model: m.id,
                        enabled_models: enabled.includes(m.id) ? enabled : [...enabled, m.id],
                      })
                    }}
                    className={`text-xs px-2.5 py-1.5 rounded border transition-colors ${
                      config.agent.model === m.id
                        ? 'border-emerald-500 bg-emerald-900/40 text-emerald-300'
                        : 'border-zinc-700 bg-zinc-800 text-zinc-400 hover:border-zinc-500 hover:text-zinc-300'
                    }`}
                  >
                    <span className="font-mono">{m.label}</span>
                    <span className="text-zinc-600 ml-1.5">· {m.desc}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {!hasAnthropicKey && !hasOpenAIKey && (
            <p className="text-xs text-amber-400 mt-1.5">
              ⚠️ No API keys configured. Add a key in <span className="font-medium">Settings → API Keys</span> to see available models.
            </p>
          )}

          <p className="text-xs text-zinc-600 mt-1">
            Or type any model ID directly. Models from providers without a configured key won't work.
          </p>
        </div>

        {/* Model Registry */}
        {availableModels.length > 0 && (
          <div className="space-y-2">
            <label className="block text-sm font-medium text-zinc-300">Model Registry</label>
            <p className="text-xs text-zinc-500">
              Enable models for multi-LLM routing. The default model is always enabled.
            </p>
            <div className="space-y-1.5">
              {availableModels.map(m => {
                const isDefault = config.agent.model === m.id
                const enabled = isModelEnabled(m.id)
                return (
                  <div
                    key={m.id}
                    className={`flex items-center justify-between px-3 py-2.5 rounded-lg border transition-colors ${
                      enabled
                        ? 'border-zinc-600 bg-zinc-800/80'
                        : 'border-zinc-700/50 bg-zinc-900/40'
                    }`}
                  >
                    <div className="flex items-center gap-3 min-w-0">
                      <span className={`text-[10px] uppercase tracking-wider font-medium px-1.5 py-0.5 rounded ${
                        m.provider === 'Anthropic'
                          ? 'bg-[#39FF14]/10 text-[#39FF14]/70'
                          : 'bg-emerald-900/40 text-emerald-400/70'
                      }`}>
                        {m.provider}
                      </span>
                      <div className="min-w-0">
                        <span className="text-sm text-white font-mono">{m.label}</span>
                        <span className="text-xs text-zinc-500 ml-2">{m.desc}</span>
                        {isDefault && (
                          <span className="text-[10px] text-[#39FF14] ml-2 uppercase tracking-wider font-medium">default</span>
                        )}
                      </div>
                    </div>
                    <button
                      type="button"
                      onClick={() => toggleModel(m.id)}
                      disabled={isDefault}
                      className={`relative w-9 h-5 rounded-full transition-colors flex-shrink-0 ${
                        enabled
                          ? 'bg-[#39FF14]/60'
                          : 'bg-zinc-600'
                      } ${isDefault ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'}`}
                    >
                      <span className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                        enabled ? 'translate-x-4' : 'translate-x-0'
                      }`} />
                    </button>
                  </div>
                )
              })}
            </div>
          </div>
        )}

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
            className="bg-[#39FF14] hover:bg-[#2bcc10] disabled:opacity-40 text-black text-sm px-5 py-2 rounded-md transition-colors"
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
            className="text-sm text-[#39FF14] hover:text-[#39FF14] disabled:opacity-40 transition-colors"
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
