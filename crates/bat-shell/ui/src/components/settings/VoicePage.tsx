import { useState, useEffect } from 'react'
import type { BatConfig, ElevenLabsVoice } from '../../types'
import { getConfig, updateConfig, fetchElevenlabsVoices } from '../../lib/tauri'

export function VoicePage() {
  const [config, setConfig] = useState<BatConfig | null>(null)
  const [saved, setSaved] = useState(false)

  // Voice state
  const [ttsEnabled, setTtsEnabled] = useState(false)
  const [ttsProvider, setTtsProvider] = useState('openai')
  const [openaiVoice, setOpenaiVoice] = useState('nova')
  const [openaiTtsModel, setOpenaiTtsModel] = useState('gpt-4o-mini-tts')
  const [elevenlabsVoiceId, setElevenlabsVoiceId] = useState('')
  const [sttEnabled, setSttEnabled] = useState(false)

  // ElevenLabs voice list (fetched from API)
  const [elVoices, setElVoices] = useState<ElevenLabsVoice[]>([])
  const [elLoading, setElLoading] = useState(false)
  const [elError, setElError] = useState<string | null>(null)

  // Derived: do we have the right keys?
  const hasOpenAIKey = !!(config?.api_keys?.openai)
  const hasElevenlabsKey = !!(config?.api_keys?.elevenlabs)
  const hasAnyTtsKey = hasOpenAIKey || hasElevenlabsKey

  useEffect(() => {
    getConfig().then(cfg => {
      setConfig(cfg)
      const v = cfg.voice
      if (v) {
        setTtsEnabled(v.tts_enabled)
        setTtsProvider(v.tts_provider || 'openai')
        setOpenaiVoice(v.openai_voice || 'nova')
        setOpenaiTtsModel(v.openai_tts_model || 'gpt-4o-mini-tts')
        setElevenlabsVoiceId(v.elevenlabs_voice_id || '')
        setSttEnabled(v.stt_enabled)
      }
    })
  }, [])

  // Fetch ElevenLabs voices when provider is selected and key exists
  useEffect(() => {
    if (ttsProvider === 'elevenlabs' && hasElevenlabsKey && elVoices.length === 0) {
      setElLoading(true)
      setElError(null)
      fetchElevenlabsVoices()
        .then(voices => {
          setElVoices(voices)
          // Auto-select first voice if none set
          if (!elevenlabsVoiceId && voices.length > 0) {
            setElevenlabsVoiceId(voices[0].voiceId)
          }
        })
        .catch(e => setElError(String(e)))
        .finally(() => setElLoading(false))
    }
  }, [ttsProvider, hasElevenlabsKey])

  // Auto-switch provider if current provider has no key
  useEffect(() => {
    if (config) {
      if (ttsProvider === 'openai' && !hasOpenAIKey && hasElevenlabsKey) {
        setTtsProvider('elevenlabs')
      } else if (ttsProvider === 'elevenlabs' && !hasElevenlabsKey && hasOpenAIKey) {
        setTtsProvider('openai')
      }
    }
  }, [config])

  const handleSave = async () => {
    if (!config) return
    const updated: BatConfig = {
      ...config,
      voice: {
        tts_enabled: ttsEnabled,
        tts_provider: ttsProvider,
        openai_voice: openaiVoice,
        openai_tts_model: openaiTtsModel,
        elevenlabs_voice_id: elevenlabsVoiceId || null,
        stt_enabled: sttEnabled,
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
      {/* No keys at all */}
      {!hasAnyTtsKey && (
        <div className="bg-amber-900/20 border border-amber-700/30 rounded-lg px-4 py-3">
          <p className="text-xs text-amber-300">
            ⚠️ No voice-capable API keys configured. Add an <span className="font-medium">OpenAI</span> or{' '}
            <span className="font-medium">ElevenLabs</span> key in{' '}
            <span className="font-medium">Settings → API Keys</span> to enable voice features.
          </p>
        </div>
      )}

      {/* Text-to-Speech */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-semibold text-zinc-200">Text-to-Speech</h3>
            <p className="text-xs text-zinc-500 mt-0.5">Send voice responses via Telegram</p>
          </div>
          <button
            onClick={() => setTtsEnabled(!ttsEnabled)}
            disabled={!hasAnyTtsKey}
            className={`relative w-10 h-5 rounded-full transition-colors ${
              ttsEnabled ? 'bg-indigo-600' : 'bg-zinc-700'
            } ${!hasAnyTtsKey ? 'opacity-40 cursor-not-allowed' : ''}`}
          >
            <span className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
              ttsEnabled ? 'translate-x-5' : ''
            }`} />
          </button>
        </div>

        {ttsEnabled && (
          <div className="space-y-3 pl-0.5">
            {/* Provider picker */}
            <div>
              <label className="text-xs text-zinc-400 font-medium">Provider</label>
              <div className="mt-1 flex gap-2">
                <button
                  onClick={() => setTtsProvider('openai')}
                  disabled={!hasOpenAIKey}
                  className={`flex-1 px-3 py-2 rounded-lg border text-sm transition-colors ${
                    ttsProvider === 'openai'
                      ? 'border-indigo-500 bg-indigo-900/40 text-indigo-300'
                      : hasOpenAIKey
                        ? 'border-zinc-700 bg-zinc-800 text-zinc-300 hover:border-zinc-500'
                        : 'border-zinc-800 bg-zinc-900/50 text-zinc-600 cursor-not-allowed'
                  }`}
                >
                  <div className="font-medium">OpenAI</div>
                  <div className="text-[10px] mt-0.5 opacity-70">
                    {hasOpenAIKey ? '6 voices · 3 models' : '🔒 Add key in API Keys'}
                  </div>
                </button>
                <button
                  onClick={() => setTtsProvider('elevenlabs')}
                  disabled={!hasElevenlabsKey}
                  className={`flex-1 px-3 py-2 rounded-lg border text-sm transition-colors ${
                    ttsProvider === 'elevenlabs'
                      ? 'border-indigo-500 bg-indigo-900/40 text-indigo-300'
                      : hasElevenlabsKey
                        ? 'border-zinc-700 bg-zinc-800 text-zinc-300 hover:border-zinc-500'
                        : 'border-zinc-800 bg-zinc-900/50 text-zinc-600 cursor-not-allowed'
                  }`}
                >
                  <div className="font-medium">ElevenLabs</div>
                  <div className="text-[10px] mt-0.5 opacity-70">
                    {hasElevenlabsKey ? 'Premium · custom voices' : '🔒 Add key in API Keys'}
                  </div>
                </button>
              </div>
            </div>

            {/* OpenAI options */}
            {ttsProvider === 'openai' && (
              <>
                <div>
                  <label className="text-xs text-zinc-400 font-medium">Model</label>
                  <select
                    value={openaiTtsModel}
                    onChange={e => setOpenaiTtsModel(e.target.value)}
                    className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white outline-none focus:border-indigo-500"
                  >
                    <option value="gpt-4o-mini-tts">gpt-4o-mini-tts (fastest, cheapest)</option>
                    <option value="tts-1">tts-1 (standard)</option>
                    <option value="tts-1-hd">tts-1-hd (high quality)</option>
                  </select>
                </div>
                <div>
                  <label className="text-xs text-zinc-400 font-medium">Voice</label>
                  <select
                    value={openaiVoice}
                    onChange={e => setOpenaiVoice(e.target.value)}
                    className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white outline-none focus:border-indigo-500"
                  >
                    <option value="alloy">Alloy (neutral)</option>
                    <option value="echo">Echo (male)</option>
                    <option value="fable">Fable (British)</option>
                    <option value="nova">Nova (female)</option>
                    <option value="onyx">Onyx (deep male)</option>
                    <option value="shimmer">Shimmer (expressive)</option>
                  </select>
                </div>
              </>
            )}

            {/* ElevenLabs options */}
            {ttsProvider === 'elevenlabs' && (
              <div>
                <label className="text-xs text-zinc-400 font-medium">Voice</label>
                {elLoading ? (
                  <div className="mt-1 text-xs text-zinc-500">Loading voices...</div>
                ) : elError ? (
                  <div className="mt-1 space-y-1">
                    <p className="text-xs text-red-400">Failed to fetch voices: {elError}</p>
                    <input
                      value={elevenlabsVoiceId}
                      onChange={e => setElevenlabsVoiceId(e.target.value)}
                      placeholder="Enter voice ID manually"
                      className="w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white placeholder-zinc-600 outline-none focus:border-indigo-500 font-mono"
                    />
                  </div>
                ) : elVoices.length > 0 ? (
                  <select
                    value={elevenlabsVoiceId}
                    onChange={e => setElevenlabsVoiceId(e.target.value)}
                    className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white outline-none focus:border-indigo-500"
                  >
                    {/* Group by category */}
                    {['cloned', 'generated', 'premade'].map(cat => {
                      const catVoices = elVoices.filter(v => v.category === cat)
                      if (catVoices.length === 0) return null
                      return (
                        <optgroup key={cat} label={cat.charAt(0).toUpperCase() + cat.slice(1)}>
                          {catVoices.map(v => (
                            <option key={v.voiceId} value={v.voiceId}>
                              {v.name}
                            </option>
                          ))}
                        </optgroup>
                      )
                    })}
                    {/* Voices without recognized category */}
                    {elVoices.filter(v => !['cloned', 'generated', 'premade'].includes(v.category)).map(v => (
                      <option key={v.voiceId} value={v.voiceId}>
                        {v.name}
                      </option>
                    ))}
                  </select>
                ) : (
                  <input
                    value={elevenlabsVoiceId}
                    onChange={e => setElevenlabsVoiceId(e.target.value)}
                    placeholder="pFZP5JQG7iQjIQuC4Bku"
                    className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white placeholder-zinc-600 outline-none focus:border-indigo-500 font-mono"
                  />
                )}
                <p className="text-[10px] text-zinc-600 mt-1">
                  Manage voices at{' '}
                  <a href="https://elevenlabs.io/voice-library" className="text-indigo-400 hover:underline" target="_blank">
                    ElevenLabs Voice Library
                  </a>
                </p>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Speech-to-Text */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-semibold text-zinc-200">Speech-to-Text</h3>
            <p className="text-xs text-zinc-500 mt-0.5">Transcribe incoming voice messages via Whisper</p>
          </div>
          <button
            onClick={() => setSttEnabled(!sttEnabled)}
            disabled={!hasOpenAIKey}
            className={`relative w-10 h-5 rounded-full transition-colors ${
              sttEnabled ? 'bg-indigo-600' : 'bg-zinc-700'
            } ${!hasOpenAIKey ? 'opacity-40 cursor-not-allowed' : ''}`}
          >
            <span className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
              sttEnabled ? 'translate-x-5' : ''
            }`} />
          </button>
        </div>
        {!hasOpenAIKey && (
          <p className="text-xs text-zinc-600 pl-0.5">
            Requires an OpenAI key — add one in <span className="font-medium">API Keys</span>.
          </p>
        )}
      </div>

      {/* Save */}
      <button
        onClick={handleSave}
        className={`px-4 py-1.5 rounded text-sm font-medium transition-colors ${
          saved ? 'bg-emerald-600 text-white' : 'bg-indigo-600 hover:bg-indigo-500 text-white'
        }`}
      >
        {saved ? '✓ Saved' : 'Save Changes'}
      </button>
    </div>
  )
}
