import { useState, useEffect } from 'react'
import type { BatConfig } from '../../types'
import { getConfig, updateConfig } from '../../lib/tauri'

const TEMPLATES: { label: string; icon: string; prompt: string }[] = [
  {
    label: 'Professional',
    icon: '💼',
    prompt: `You are a focused, professional assistant. Communicate clearly and concisely. Prioritize accuracy and efficiency. Avoid casual language, jokes, or unnecessary commentary. Structure your responses logically and get straight to the point.`,
  },
  {
    label: 'Friendly',
    icon: '😊',
    prompt: `You are a warm, friendly assistant who genuinely enjoys helping. Be conversational and approachable. Use casual language when appropriate, add encouraging words, and show enthusiasm. Make the user feel comfortable asking anything.`,
  },
  {
    label: 'Snarky',
    icon: '😏',
    prompt: `You are a witty, sarcastic assistant with a dry sense of humor. You get the job done but can't resist a clever quip or playful jab. Keep it fun and never mean-spirited — think helpful friend who roasts you affectionately.`,
  },
  {
    label: 'Creative',
    icon: '🎨',
    prompt: `You are an imaginative, creative assistant who thinks outside the box. Use vivid language, metaphors, and unexpected angles. Suggest novel approaches to problems. Be expressive and bring energy and originality to every interaction.`,
  },
]

export function PersonalityPage() {
  const [config, setConfig] = useState<BatConfig | null>(null)
  const [name, setName] = useState('')
  const [personalityPrompt, setPersonalityPrompt] = useState('')
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    getConfig().then(cfg => {
      setConfig(cfg)
      setName(cfg.agent.name)
      setPersonalityPrompt(cfg.agent.personality_prompt || '')
    })
  }, [])

  const handleSave = async () => {
    if (!config) return
    setError(null)
    setSaved(false)
    try {
      const updated: BatConfig = {
        ...config,
        agent: {
          ...config.agent,
          name,
          personality_prompt: personalityPrompt.trim() || null,
        },
      }
      await updateConfig(updated)
      setConfig(updated)
      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch (e) {
      setError(String(e))
    }
  }

  if (!config) return <div className="p-4 text-zinc-500">Loading...</div>

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-white">Personality</h2>
        <p className="text-sm text-zinc-400 mt-1">
          Customize how the agent communicates and behaves.
        </p>
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-700 rounded-lg px-4 py-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      {saved && (
        <div className="bg-green-900/30 border border-green-700 rounded-lg px-4 py-3 text-green-300 text-sm">
          Personality saved — takes effect on the next message.
        </div>
      )}

      {/* Agent Name */}
      <div className="space-y-1.5">
        <label className="block text-sm font-medium text-zinc-300">Agent Name</label>
        <input
          type="text"
          value={name}
          onChange={e => setName(e.target.value)}
          className="w-full bg-zinc-900 border border-zinc-600 rounded-md px-3 py-2 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-400"
          placeholder="Aria"
        />
        <p className="text-xs text-zinc-500">The name used in the system prompt and greetings.</p>
      </div>

      {/* Starter Templates */}
      <div className="space-y-2">
        <label className="block text-sm font-medium text-zinc-300">Quick Templates</label>
        <div className="flex flex-wrap gap-2">
          {TEMPLATES.map(t => (
            <button
              key={t.label}
              type="button"
              onClick={() => setPersonalityPrompt(t.prompt)}
              className={`text-xs px-3 py-2 rounded-lg border transition-colors ${
                personalityPrompt === t.prompt
                  ? 'border-[#39FF14] bg-[#39FF14]/10 text-[#39FF14]'
                  : 'border-zinc-700 bg-zinc-800 text-zinc-400 hover:border-zinc-500 hover:text-zinc-300'
              }`}
            >
              <span className="mr-1.5">{t.icon}</span>
              {t.label}
            </button>
          ))}
        </div>
        <p className="text-xs text-zinc-500">Pick a starting point, then customize below.</p>
      </div>

      {/* Personality Prompt */}
      <div className="space-y-1.5">
        <label className="block text-sm font-medium text-zinc-300">Personality Prompt</label>
        <textarea
          value={personalityPrompt}
          onChange={e => setPersonalityPrompt(e.target.value)}
          rows={6}
          className="w-full bg-zinc-900 border border-zinc-600 rounded-md px-3 py-2 text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-400 resize-y font-mono leading-relaxed"
          placeholder="Describe the agent's personality, tone, and communication style..."
        />
        <p className="text-xs text-zinc-500">
          This is written to <span className="font-mono text-zinc-400">IDENTITY.md</span> and injected into the system prompt.
          Leave blank to use no personality override.
        </p>
      </div>

      {/* Save */}
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
