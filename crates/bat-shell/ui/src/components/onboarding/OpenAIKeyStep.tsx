interface Props {
  apiKey: string
  setApiKey: (key: string) => void
  onNext: () => void
  onBack: () => void
}

export function OpenAIKeyStep({ apiKey, setApiKey, onNext, onBack }: Props) {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-bold text-white">OpenAI API Key</h2>
        <p className="text-sm text-zinc-400 mt-2">
          This is <span className="text-zinc-200 font-medium">optional</span> — skip it if you don't need voice features.
        </p>
      </div>

      <div className="bg-zinc-800/50 border border-zinc-700 rounded-lg p-4 space-y-2">
        <p className="text-xs font-semibold text-zinc-300">What is this for?</p>
        <ul className="text-xs text-zinc-400 space-y-1.5">
          <li className="flex gap-2">
            <span>🎤</span>
            <span><span className="text-zinc-200">Voice messages</span> — Send voice notes to your agent via Telegram and have them transcribed automatically (powered by OpenAI Whisper)</span>
          </li>
          <li className="flex gap-2">
            <span>🔊</span>
            <span><span className="text-zinc-200">Voice responses</span> — Your agent can reply with voice messages using OpenAI's text-to-speech</span>
          </li>
        </ul>
        <p className="text-[10px] text-zinc-500 mt-2">
          These features use OpenAI's APIs, which require a separate API key from Anthropic.
          You can always add this later in Settings → Voice.
        </p>
      </div>

      <div>
        <label className="text-xs text-zinc-400 font-medium">OpenAI API Key</label>
        <input
          type="password"
          value={apiKey}
          onChange={e => setApiKey(e.target.value)}
          placeholder="sk-..."
          className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded-lg px-4 py-2.5 text-sm text-white
                     placeholder-zinc-600 outline-none focus:border-indigo-500 transition-colors"
        />
        <p className="text-[10px] text-zinc-600 mt-1.5">
          Get one at{' '}
          <a href="https://platform.openai.com/api-keys" className="text-indigo-400 hover:underline" target="_blank">
            platform.openai.com/api-keys
          </a>
        </p>
      </div>

      <div className="flex justify-between pt-2">
        <button
          onClick={onBack}
          className="px-4 py-2 text-sm text-zinc-400 hover:text-zinc-200 transition-colors"
        >
          ← Back
        </button>
        <button
          onClick={onNext}
          className="px-6 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-sm font-medium rounded-lg transition-colors"
        >
          {apiKey.trim() ? 'Next →' : 'Skip →'}
        </button>
      </div>
    </div>
  )
}
