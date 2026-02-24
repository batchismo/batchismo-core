import { useState } from 'react'
import { validateApiKey } from '../../lib/tauri'

interface Props {
  apiKey: string
  setApiKey: (key: string) => void
  onNext: () => void
  onBack: () => void
}

export function ApiKeyStep({ apiKey, setApiKey, onNext, onBack }: Props) {
  const [showKey, setShowKey] = useState(false)
  const [validating, setValidating] = useState(false)
  const [validated, setValidated] = useState(false)
  const [error, setError] = useState('')

  async function handleValidate() {
    if (!apiKey.trim()) return
    setValidating(true)
    setError('')
    setValidated(false)
    try {
      await validateApiKey(apiKey.trim())
      setValidated(true)
    } catch (e) {
      setError(String(e))
    } finally {
      setValidating(false)
    }
  }

  return (
    <div>
      <h2 className="text-xl font-bold text-white mb-2">Connect to an AI Provider</h2>
      <p className="text-zinc-400 text-sm mb-6">
        Batchismo uses Anthropic's Claude to power your agent. Enter your API key below.
        You can get one at{' '}
        <span className="text-[#39FF14]">console.anthropic.com</span>.
      </p>

      <div className="mb-4">
        <label className="block text-sm text-zinc-400 mb-1.5">Anthropic API Key</label>
        <div className="flex gap-2">
          <div className="relative flex-1">
            <input
              type={showKey ? 'text' : 'password'}
              value={apiKey}
              onChange={e => { setApiKey(e.target.value); setValidated(false); setError('') }}
              placeholder="sk-ant-..."
              className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-[#39FF14] font-mono"
            />
            <button
              onClick={() => setShowKey(!showKey)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300 text-xs"
            >
              {showKey ? 'Hide' : 'Show'}
            </button>
          </div>
          <button
            onClick={handleValidate}
            disabled={!apiKey.trim() || validating}
            className="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 disabled:cursor-not-allowed text-sm rounded-lg transition-colors whitespace-nowrap"
          >
            {validating ? 'Checking...' : 'Validate'}
          </button>
        </div>

        {/* Status */}
        {validated && (
          <p className="mt-2 text-sm text-emerald-400 flex items-center gap-1">
            <span>✓</span> API key is valid
          </p>
        )}
        {error && (
          <p className="mt-2 text-sm text-red-400 flex items-center gap-1">
            <span>✗</span> {error}
          </p>
        )}
      </div>

      <div className="flex justify-between mt-8">
        <button
          onClick={onBack}
          className="px-4 py-2 text-zinc-400 hover:text-zinc-200 text-sm transition-colors"
        >
          ← Back
        </button>
        <button
          onClick={onNext}
          disabled={!validated}
          className="px-6 py-2.5 bg-[#39FF14] hover:bg-[#2bcc10] disabled:opacity-40 disabled:cursor-not-allowed text-black font-medium rounded-lg transition-colors"
        >
          Next →
        </button>
      </div>
    </div>
  )
}
