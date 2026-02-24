interface Props {
  name: string
  onFinish: () => void
  onBack: () => void
  saving: boolean
  error: string
}

export function ReadyStep({ name, onFinish, onBack, saving, error }: Props) {
  return (
    <div className="text-center">
      <div className="w-14 h-14 rounded-full bg-emerald-600/20 border border-emerald-500/30 flex items-center justify-center mx-auto mb-6">
        <span className="text-2xl">✓</span>
      </div>

      <h2 className="text-xl font-bold text-white mb-2">You're All Set!</h2>
      <p className="text-zinc-400 mb-6">
        <span className="text-[#39FF14] font-medium">{name}</span> is ready to help.
        Try asking it to list files in one of your folders, or just say hello.
      </p>

      {error && (
        <p className="text-sm text-red-400 mb-4">
          Something went wrong: {error}
        </p>
      )}

      <div className="flex justify-between mt-8">
        <button
          onClick={onBack}
          disabled={saving}
          className="px-4 py-2 text-zinc-400 hover:text-zinc-200 text-sm transition-colors disabled:opacity-50"
        >
          ← Back
        </button>
        <button
          onClick={onFinish}
          disabled={saving}
          className="px-6 py-2.5 bg-[#39FF14] hover:bg-[#2bcc10] disabled:opacity-50 text-black font-medium rounded-lg transition-colors"
        >
          {saving ? 'Setting up...' : 'Start Chatting →'}
        </button>
      </div>
    </div>
  )
}
