import { useState, useEffect } from 'react'
import type { BatConfig } from '../../types'
import { getConfig, updateConfig } from '../../lib/tauri'

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

  // Derived: do we have the right keys?
  const hasOpenAIKey = !!(config?.api_keys?.openai)
  const hasElevenlabsKey = !!(config?.api_keys?.elevenlabs)

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

  const ttsKeyMissing = ttsProvider === 'openai' ? !hasOpenAIKey : !hasElevenlabsKey

  return (
    <div className="p-4 space-y-6">
      {/* Key status banner */}
      {!hasOpenAIKey && (
        <div className="bg-amber-900/20 border border-amber-700/30 rounded-lg px-4 py-3">
          <p className="text-xs text-amber-300">
            ⚠️ No OpenAI API key configured. Voice features (TTS & STT) require an OpenAI key.
            Add one in <span className="font-medium">Settings → API Keys</span>.
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
            className={`relative w-10 h-5 rounded-full transition-colors ${
              ttsEnabled ? 'bg-indigo-600' : 'bg-zinc-700'
            }`}
          >
            <span className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
              ttsEnabled ? 'translate-x-5' : ''
            }`} />
          </button>
        </div>

        {ttsEnabled && (
          <div className="space-y-3 pl-0.5">
            {ttsKeyMissing && (
              <p className="text-xs text-amber-400">
                ⚠️ {ttsProvider === 'elevenlabs' ? 'ElevenLabs' : 'OpenAI'} API key not set — TTS won't work until you add it in API Keys.
              </p>
            )}
            <div>
              <label className="text-xs text-zinc-400 font-medium">Provider</label>
              <select
                value={ttsProvider}
                onChange={e => setTtsProvider(e.target.value)}
                className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white outline-none focus:border-indigo-500"
              >
                <option value="openai">OpenAI</option>
                <option value="elevenlabs">ElevenLabs</option>
              </select>
            </div>

            {ttsProvider === 'openai' && (
              <>
                <div>
                  <label className="text-xs text-zinc-400 font-medium">Model</label>
                  <select
                    value={openaiTtsModel}
                    onChange={e => setOpenaiTtsModel(e.target.value)}
                    className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white outline-none focus:border-indigo-500"
                  >
                    <option value="gpt-4o-mini-tts">gpt-4o-mini-tts (fast, cheap)</option>
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
                    <option value="alloy">Alloy</option>
                    <option value="echo">Echo</option>
                    <option value="fable">Fable</option>
                    <option value="nova">Nova</option>
                    <option value="onyx">Onyx</option>
                    <option value="shimmer">Shimmer</option>
                  </select>
                </div>
              </>
            )}

            {ttsProvider === 'elevenlabs' && (
              <div>
                <label className="text-xs text-zinc-400 font-medium">Voice ID</label>
                <input
                  value={elevenlabsVoiceId}
                  onChange={e => setElevenlabsVoiceId(e.target.value)}
                  placeholder="pFZP5JQG7iQjIQuC4Bku"
                  className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white placeholder-zinc-600 outline-none focus:border-indigo-500"
                />
                <p className="text-[10px] text-zinc-600 mt-1">
                  Find voice IDs at <a href="https://elevenlabs.io/voice-library" className="text-indigo-400 hover:underline" target="_blank">ElevenLabs Voice Library</a>
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
            <p className="text-xs text-zinc-500 mt-0.5">Transcribe incoming voice messages via Whisper (requires OpenAI key)</p>
          </div>
          <button
            onClick={() => setSttEnabled(!sttEnabled)}
            className={`relative w-10 h-5 rounded-full transition-colors ${
              sttEnabled ? 'bg-indigo-600' : 'bg-zinc-700'
            }`}
          >
            <span className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
              sttEnabled ? 'translate-x-5' : ''
            }`} />
          </button>
        </div>
        {sttEnabled && !hasOpenAIKey && (
          <p className="text-xs text-amber-400 pl-0.5">
            ⚠️ OpenAI API key not set — STT won't work until you add it in API Keys.
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
