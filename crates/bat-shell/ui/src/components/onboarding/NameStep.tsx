interface Props {
  name: string
  setName: (name: string) => void
  onNext: () => void
  onBack: () => void
}

const SUGGESTIONS = ['Aria', 'Atlas', 'Sage', 'Nova', 'Echo', 'Cipher']

export function NameStep({ name, setName, onNext, onBack }: Props) {
  return (
    <div>
      <h2 className="text-xl font-bold text-white mb-2">Name Your Agent</h2>
      <p className="text-zinc-400 text-sm mb-6">
        Give your AI agent a name. This is how it'll identify itself.
      </p>

      <div className="mb-4">
        <label className="block text-sm text-zinc-400 mb-1.5">Agent Name</label>
        <input
          type="text"
          value={name}
          onChange={e => setName(e.target.value)}
          placeholder="Enter a name..."
          className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-indigo-500"
          autoFocus
        />
      </div>

      {/* Suggestions */}
      <div className="flex flex-wrap gap-2 mb-6">
        {SUGGESTIONS.map(s => (
          <button
            key={s}
            onClick={() => setName(s)}
            className={`px-3 py-1 text-xs rounded-full border transition-colors ${
              name === s
                ? 'border-indigo-500 bg-indigo-600/20 text-indigo-300'
                : 'border-zinc-700 text-zinc-400 hover:border-zinc-600 hover:text-zinc-300'
            }`}
          >
            {s}
          </button>
        ))}
      </div>

      {/* Preview */}
      {name.trim() && (
        <div className="bg-zinc-800/50 border border-zinc-700/50 rounded-lg px-4 py-3 mb-6">
          <p className="text-sm text-zinc-300">
            Meet <span className="text-indigo-400 font-medium">{name.trim()}</span>, your personal AI agent.
          </p>
        </div>
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
          disabled={!name.trim()}
          className="px-6 py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors"
        >
          Next →
        </button>
      </div>
    </div>
  )
}
