import { useState } from 'react'
import { validateApiKey, validateOpenaiKey, ollamaStatus } from '../../lib/tauri'

export type LLMProvider = 'anthropic' | 'openai' | 'ollama'

interface Props {
  selectedProvider: LLMProvider
  setSelectedProvider: (provider: LLMProvider) => void
  anthropicKey: string
  setAnthropicKey: (key: string) => void
  openaiKey: string
  setOpenaiKey: (key: string) => void
  onNext: () => void
  onBack: () => void
}

export function ProviderStep({ 
  selectedProvider, 
  setSelectedProvider, 
  anthropicKey, 
  setAnthropicKey,
  openaiKey,
  setOpenaiKey,
  onNext, 
  onBack 
}: Props) {
  const [showAnthropicKey, setShowAnthropicKey] = useState(false)
  const [showOpenaiKey, setShowOpenaiKey] = useState(false)
  const [validatingAnthropic, setValidatingAnthropic] = useState(false)
  const [validatingOpenai, setValidatingOpenai] = useState(false)
  const [anthropicValidated, setAnthropicValidated] = useState(false)
  const [openaiValidated, setOpenaiValidated] = useState(false)
  const [checkingOllama, setCheckingOllama] = useState(false)
  const [ollamaAvailable, setOllamaAvailable] = useState(false)
  const [error, setError] = useState('')

  async function handleValidateAnthropic() {
    if (!anthropicKey.trim()) return
    setValidatingAnthropic(true)
    setError('')
    try {
      await validateApiKey(anthropicKey.trim())
      setAnthropicValidated(true)
    } catch (e) {
      setError(String(e))
    } finally {
      setValidatingAnthropic(false)
    }
  }

  async function handleValidateOpenai() {
    if (!openaiKey.trim()) return
    setValidatingOpenai(true)
    setError('')
    try {
      await validateOpenaiKey(openaiKey.trim())
      setOpenaiValidated(true)
    } catch (e) {
      setError(String(e))
    } finally {
      setValidatingOpenai(false)
    }
  }

  async function handleCheckOllama() {
    setCheckingOllama(true)
    setError('')
    try {
      const available = await ollamaStatus()
      setOllamaAvailable(available)
      if (available) {
        setSelectedProvider('ollama')
      } else {
        setError('Ollama is not running. Please start Ollama and try again.')
      }
    } catch (e) {
      setError('Could not connect to Ollama. Please make sure it\'s installed and running.')
    } finally {
      setCheckingOllama(false)
    }
  }

  const canProceed = 
    (selectedProvider === 'anthropic' && anthropicValidated) ||
    (selectedProvider === 'openai' && openaiValidated) ||
    (selectedProvider === 'ollama' && ollamaAvailable)

  return (
    <div>
      <h2 className="text-xl font-bold text-white mb-2">Choose Your AI Provider</h2>
      <p className="text-zinc-400 text-sm mb-6">
        Select how you want to power your AI agent. You can change this later in Settings.
      </p>

      <div className="space-y-3 mb-6">
        {/* Anthropic (Recommended) */}
        <div 
          className={`border rounded-xl p-4 cursor-pointer transition-colors ${
            selectedProvider === 'anthropic' 
              ? 'border-[#39FF14] bg-[#39FF14]/5' 
              : 'border-zinc-700 hover:border-zinc-600'
          }`}
          onClick={() => setSelectedProvider('anthropic')}
        >
          <div className="flex items-center gap-3 mb-2">
            <input 
              type="radio" 
              checked={selectedProvider === 'anthropic'}
              onChange={() => setSelectedProvider('anthropic')}
              className="w-4 h-4" 
            />
            <h3 className="font-medium text-white flex items-center gap-2">
              Anthropic Claude
              <span className="px-2 py-0.5 bg-[#39FF14]/20 text-[#39FF14] text-xs rounded border border-[#39FF14]/30">
                Recommended
              </span>
            </h3>
          </div>
          <p className="text-sm text-zinc-400 mb-3">
            Best reasoning and tool use. Requires API key from console.anthropic.com
          </p>
          
          {selectedProvider === 'anthropic' && (
            <div className="space-y-2">
              <div className="flex gap-2">
                <div className="relative flex-1">
                  <input
                    type={showAnthropicKey ? 'text' : 'password'}
                    value={anthropicKey}
                    onChange={e => { setAnthropicKey(e.target.value); setAnthropicValidated(false); setError('') }}
                    placeholder="sk-ant-..."
                    className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-[#39FF14] font-mono"
                  />
                  <button
                    onClick={() => setShowAnthropicKey(!showAnthropicKey)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300 text-xs"
                  >
                    {showAnthropicKey ? 'Hide' : 'Show'}
                  </button>
                </div>
                <button
                  onClick={handleValidateAnthropic}
                  disabled={!anthropicKey.trim() || validatingAnthropic}
                  className="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 disabled:cursor-not-allowed text-sm rounded-lg transition-colors whitespace-nowrap"
                >
                  {validatingAnthropic ? 'Checking...' : 'Validate'}
                </button>
              </div>
              {anthropicValidated && (
                <p className="text-sm text-emerald-400 flex items-center gap-1">
                  <span>✓</span> API key is valid
                </p>
              )}
            </div>
          )}
        </div>

        {/* OpenAI */}
        <div 
          className={`border rounded-xl p-4 cursor-pointer transition-colors ${
            selectedProvider === 'openai' 
              ? 'border-[#39FF14] bg-[#39FF14]/5' 
              : 'border-zinc-700 hover:border-zinc-600'
          }`}
          onClick={() => setSelectedProvider('openai')}
        >
          <div className="flex items-center gap-3 mb-2">
            <input 
              type="radio" 
              checked={selectedProvider === 'openai'}
              onChange={() => setSelectedProvider('openai')}
              className="w-4 h-4" 
            />
            <h3 className="font-medium text-white">OpenAI GPT</h3>
          </div>
          <p className="text-sm text-zinc-400 mb-3">
            GPT-4o and other OpenAI models. Requires API key from platform.openai.com
          </p>
          
          {selectedProvider === 'openai' && (
            <div className="space-y-2">
              <div className="flex gap-2">
                <div className="relative flex-1">
                  <input
                    type={showOpenaiKey ? 'text' : 'password'}
                    value={openaiKey}
                    onChange={e => { setOpenaiKey(e.target.value); setOpenaiValidated(false); setError('') }}
                    placeholder="sk-..."
                    className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-[#39FF14] font-mono"
                  />
                  <button
                    onClick={() => setShowOpenaiKey(!showOpenaiKey)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300 text-xs"
                  >
                    {showOpenaiKey ? 'Hide' : 'Show'}
                  </button>
                </div>
                <button
                  onClick={handleValidateOpenai}
                  disabled={!openaiKey.trim() || validatingOpenai}
                  className="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 disabled:cursor-not-allowed text-sm rounded-lg transition-colors whitespace-nowrap"
                >
                  {validatingOpenai ? 'Checking...' : 'Validate'}
                </button>
              </div>
              {openaiValidated && (
                <p className="text-sm text-emerald-400 flex items-center gap-1">
                  <span>✓</span> API key is valid
                </p>
              )}
            </div>
          )}
        </div>

        {/* Ollama (Local) */}
        <div 
          className={`border rounded-xl p-4 cursor-pointer transition-colors ${
            selectedProvider === 'ollama' 
              ? 'border-[#39FF14] bg-[#39FF14]/5' 
              : 'border-zinc-700 hover:border-zinc-600'
          }`}
          onClick={() => setSelectedProvider('ollama')}
        >
          <div className="flex items-center gap-3 mb-2">
            <input 
              type="radio" 
              checked={selectedProvider === 'ollama'}
              onChange={() => setSelectedProvider('ollama')}
              className="w-4 h-4" 
            />
            <h3 className="font-medium text-white flex items-center gap-2">
              Ollama (Local)
              <span className="px-2 py-0.5 bg-blue-900/20 text-blue-400 text-xs rounded border border-blue-600/30">
                Private
              </span>
            </h3>
          </div>
          <p className="text-sm text-zinc-400 mb-3">
            Run models locally on your machine. Requires Ollama to be installed and running.
          </p>
          
          {selectedProvider === 'ollama' && (
            <div className="space-y-2">
              <button
                onClick={handleCheckOllama}
                disabled={checkingOllama}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-sm rounded-lg transition-colors"
              >
                {checkingOllama ? 'Checking...' : 'Check Ollama Connection'}
              </button>
              {ollamaAvailable && (
                <p className="text-sm text-emerald-400 flex items-center gap-1">
                  <span>✓</span> Ollama is available
                </p>
              )}
            </div>
          )}
        </div>
      </div>

      {error && (
        <p className="text-sm text-red-400 mb-4 flex items-center gap-1">
          <span>✗</span> {error}
        </p>
      )}

      <div className="flex justify-between mt-8">
        <button
          onClick={onBack}
          className="px-4 py-2 text-zinc-400 hover:text-zinc-200 text-sm transition-colors"
        >
          ← Back
        </button>
        <button
          onClick={onNext}
          disabled={!canProceed}
          className="px-6 py-2.5 bg-[#39FF14] hover:bg-[#2bcc10] disabled:opacity-40 disabled:cursor-not-allowed text-black font-medium rounded-lg transition-colors"
        >
          Next →
        </button>
      </div>
    </div>
  )
}