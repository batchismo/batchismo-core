import { useState, useEffect } from 'react'
import type { BatConfig } from '../../types'
import { getConfig, updateConfig } from '../../lib/tauri'

interface KeyField {
  id: 'anthropic' | 'openai' | 'elevenlabs'
  label: string
  description: string
  placeholder: string
  helpUrl: string
  helpLabel: string
}

const KEYS: KeyField[] = [
  {
    id: 'anthropic',
    label: 'Anthropic',
    description: 'Unlocks: Claude chat models (Sonnet, Opus, Haiku)',
    placeholder: 'sk-ant-...',
    helpUrl: 'https://console.anthropic.com/settings/keys',
    helpLabel: 'console.anthropic.com',
  },
  {
    id: 'openai',
    label: 'OpenAI',
    description: 'Unlocks: GPT chat models, voice responses (TTS), speech-to-text (Whisper)',
    placeholder: 'sk-...',
    helpUrl: 'https://platform.openai.com/api-keys',
    helpLabel: 'platform.openai.com',
  },
  {
    id: 'elevenlabs',
    label: 'ElevenLabs',
    description: 'Unlocks: Premium TTS voices, custom & cloned voices',
    placeholder: 'sk_...',
    helpUrl: 'https://elevenlabs.io/app/settings/api-keys',
    helpLabel: 'elevenlabs.io',
  },
]

export function ApiKeysPage() {
  const [config, setConfig] = useState<BatConfig | null>(null)
  const [keys, setKeys] = useState<Record<string, string>>({})
  const [showKey, setShowKey] = useState<Record<string, boolean>>({})
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    getConfig().then(cfg => {
      setConfig(cfg)
      setKeys({
        anthropic: cfg.api_keys?.anthropic || '',
        openai: cfg.api_keys?.openai || '',
        elevenlabs: cfg.api_keys?.elevenlabs || '',
      })
    })
  }, [])

  const handleSave = async () => {
    if (!config) return
    const updated: BatConfig = {
      ...config,
      api_keys: {
        anthropic: keys.anthropic || null,
        openai: keys.openai || null,
        elevenlabs: keys.elevenlabs || null,
      },
      // Keep legacy field in sync
      agent: {
        ...config.agent,
        api_key: keys.anthropic || null,
      },
    }
    await updateConfig(updated)
    setConfig(updated)
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  if (!config) return <div className="p-4 text-zinc-500">Loading...</div>

  return (
    <div className="p-4 space-y-6">
      <div>
        <h3 className="text-sm font-semibold text-zinc-200">API Keys</h3>
        <p className="text-xs text-zinc-500 mt-1">
          Manage your provider API keys. Keys are stored locally in your config file and never sent anywhere except to the respective API providers.
        </p>
      </div>

      <div className="space-y-4">
        {KEYS.map(field => {
          const value = keys[field.id] || ''
          const isVisible = showKey[field.id]
          const hasKey = value.length > 0

          return (
            <div key={field.id} className="bg-zinc-900/50 border border-zinc-800 rounded-lg p-4">
              <div className="flex items-center justify-between mb-1">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-zinc-200">{field.label}</span>
                  {hasKey && (
                    <span className="text-[10px] bg-emerald-900/40 text-emerald-400 px-1.5 py-0.5 rounded font-medium">
                      Configured
                    </span>
                  )}
                </div>
              </div>
              <p className="text-xs text-zinc-500 mb-2">{field.description}</p>
              <div className="flex gap-2">
                <input
                  type={isVisible ? 'text' : 'password'}
                  value={value}
                  onChange={e => setKeys({ ...keys, [field.id]: e.target.value })}
                  placeholder={field.placeholder}
                  className="flex-1 bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white
                             placeholder-zinc-600 outline-none focus:border-[#39FF14] font-mono"
                />
                <button
                  onClick={() => setShowKey({ ...showKey, [field.id]: !isVisible })}
                  className="px-2 py-1.5 bg-zinc-800 border border-zinc-700 rounded text-xs text-zinc-400 hover:text-zinc-200"
                  title={isVisible ? 'Hide' : 'Show'}
                >
                  {isVisible ? '🙈' : '👁️'}
                </button>
              </div>
              <p className="text-[10px] text-zinc-600 mt-1.5">
                Get one at{' '}
                <a href={field.helpUrl} className="text-[#39FF14] hover:underline" target="_blank">
                  {field.helpLabel}
                </a>
              </p>
            </div>
          )
        })}
      </div>

      <button
        onClick={handleSave}
        className={`px-4 py-1.5 rounded text-sm font-medium transition-colors ${
          saved ? 'bg-emerald-600 text-white' : 'bg-[#39FF14] hover:bg-[#2bcc10] text-black'
        }`}
      >
        {saved ? '✓ Saved' : 'Save Changes'}
      </button>
    </div>
  )
}
