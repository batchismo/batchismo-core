import { useState, useEffect, useCallback } from 'react'
import type { BatConfig, OllamaModel } from '../../types'
import { getConfig, updateConfig, ollamaListModels, ollamaStatus } from '../../lib/tauri'

export function OllamaPage() {
  const [config, setConfig] = useState<BatConfig | null>(null)
  const [endpoint, setEndpoint] = useState('http://localhost:11434')
  const [connected, setConnected] = useState<boolean | null>(null)
  const [models, setModels] = useState<OllamaModel[]>([])
  const [loading, setLoading] = useState(false)
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    getConfig().then(cfg => {
      setConfig(cfg)
      setEndpoint(cfg.api_keys?.ollama_endpoint || 'http://localhost:11434')
    })
  }, [])

  const checkConnection = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const status = await ollamaStatus()
      setConnected(status)
      if (status) {
        const modelList = await ollamaListModels()
        setModels(modelList)
      } else {
        setModels([])
      }
    } catch (e) {
      setConnected(false)
      setModels([])
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (config) checkConnection()
  }, [config, checkConnection])

  const handleSave = async () => {
    if (!config) return
    const updated: BatConfig = {
      ...config,
      api_keys: {
        ...config.api_keys,
        ollama_endpoint: endpoint || null,
      },
    }
    await updateConfig(updated)
    setConfig(updated)
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
    // Re-check connection with new endpoint
    setTimeout(checkConnection, 500)
  }

  const formatSize = (bytes: number) => {
    if (bytes === 0) return '—'
    const gb = bytes / (1024 * 1024 * 1024)
    if (gb >= 1) return `${gb.toFixed(1)} GB`
    const mb = bytes / (1024 * 1024)
    return `${mb.toFixed(0)} MB`
  }

  if (!config) return <div className="p-4 text-zinc-500">Loading...</div>

  return (
    <div className="p-4 space-y-6">
      <div>
        <h3 className="text-sm font-semibold text-zinc-200">Local LLM (Ollama)</h3>
        <p className="text-xs text-zinc-500 mt-1">
          Connect to a local Ollama instance for private, offline AI inference. No API key needed.
        </p>
      </div>

      {/* Connection status */}
      <div className="flex items-center gap-2">
        <div className={`w-2 h-2 rounded-full ${
          connected === null ? 'bg-zinc-600' :
          connected ? 'bg-emerald-400' : 'bg-red-400'
        }`} />
        <span className="text-sm text-zinc-300">
          {connected === null ? 'Checking...' :
           connected ? 'Connected' : 'Disconnected'}
        </span>
        <button
          onClick={checkConnection}
          disabled={loading}
          className="text-xs text-zinc-500 hover:text-zinc-300 ml-2"
        >
          {loading ? '...' : '↻ Refresh'}
        </button>
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-700 rounded-lg px-3 py-2 text-red-300 text-xs">
          {error}
        </div>
      )}

      {/* Endpoint URL */}
      <div className="bg-zinc-900/50 border border-zinc-800 rounded-lg p-4 space-y-2">
        <label className="block text-sm font-medium text-zinc-300">Endpoint URL</label>
        <div className="flex gap-2">
          <input
            type="text"
            value={endpoint}
            onChange={e => setEndpoint(e.target.value)}
            placeholder="http://localhost:11434"
            className="flex-1 bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white
                       placeholder-zinc-600 outline-none focus:border-[#39FF14] font-mono"
          />
        </div>
        <p className="text-[10px] text-zinc-600">
          Default: http://localhost:11434 · Install Ollama from{' '}
          <a href="https://ollama.com" className="text-[#39FF14] hover:underline" target="_blank">
            ollama.com
          </a>
        </p>
      </div>

      <button
        onClick={handleSave}
        className={`px-4 py-1.5 rounded text-sm font-medium transition-colors ${
          saved ? 'bg-emerald-600 text-white' : 'bg-[#39FF14] hover:bg-[#2bcc10] text-black'
        }`}
      >
        {saved ? '✓ Saved' : 'Save Endpoint'}
      </button>

      {/* Available models */}
      {connected && (
        <div className="space-y-3">
          <div>
            <h4 className="text-sm font-medium text-zinc-300">Available Models</h4>
            <p className="text-xs text-zinc-500 mt-0.5">
              These models are installed locally and can be selected in Agent Config.
            </p>
          </div>

          {models.length === 0 ? (
            <div className="bg-zinc-900/50 border border-zinc-800 rounded-lg p-4 text-center">
              <p className="text-sm text-zinc-500">No models installed.</p>
              <p className="text-xs text-zinc-600 mt-1">
                Run <code className="bg-zinc-800 px-1.5 py-0.5 rounded font-mono">ollama pull llama3.2</code> to install one.
              </p>
            </div>
          ) : (
            <div className="space-y-1.5">
              {models.map(m => (
                <div
                  key={m.name}
                  className="flex items-center justify-between bg-zinc-900/50 border border-zinc-800 rounded-lg px-4 py-2.5"
                >
                  <div>
                    <span className="text-sm font-mono text-zinc-200">{m.name}</span>
                    {m.parameterSize && (
                      <span className="text-[10px] bg-purple-900/40 text-purple-300 px-1.5 py-0.5 rounded ml-2 font-medium">
                        {m.parameterSize}
                      </span>
                    )}
                    <span className="text-[10px] bg-zinc-800 text-zinc-400 px-1.5 py-0.5 rounded ml-1.5">
                      Local
                    </span>
                  </div>
                  <span className="text-xs text-zinc-500">{formatSize(m.size)}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
