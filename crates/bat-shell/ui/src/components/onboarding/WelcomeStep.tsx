interface Props {
  onNext: () => void
}

export function WelcomeStep({ onNext }: Props) {
  return (
    <div className="text-center">
      {/* Logo */}
      <div className="w-16 h-16 rounded-2xl bg-indigo-600/20 border border-indigo-500/30 flex items-center justify-center mx-auto mb-6">
        <svg className="w-8 h-8 text-indigo-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
      </div>

      <h1 className="text-2xl font-bold text-white mb-2">Welcome to Batchismo</h1>
      <p className="text-zinc-400 mb-6 leading-relaxed">
        Your AI that actually works on your computer. Batchismo is a personal AI agent
        that can read and write files, answer questions, and help you get things done — all
        running locally on your machine.
      </p>

      <p className="text-zinc-500 text-sm mb-8">
        Let's get you set up. It only takes a minute.
      </p>

      <button
        onClick={onNext}
        className="px-6 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-lg transition-colors"
      >
        Get Started
      </button>
    </div>
  )
}
