export function AboutPage() {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-white">About Batchismo</h2>
        <p className="text-sm text-zinc-400 mt-1">
          Your personal AI assistant, running locally on your computer.
        </p>
      </div>

      <div className="bg-zinc-800/50 border border-zinc-700 rounded-lg p-6 space-y-4">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-xl bg-[#39FF14]/20 border border-[#39FF14]/30 flex items-center justify-center">
            <svg className="w-6 h-6 text-[#39FF14]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
          <div>
            <h3 className="text-base font-semibold text-white">Batchismo</h3>
            <p className="text-sm text-zinc-400">Phase 1 — Desktop AI Agent</p>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3 text-sm">
          <div className="space-y-1">
            <p className="text-zinc-500 text-xs uppercase tracking-wider">Version</p>
            <p className="text-zinc-200">0.3.11</p>
          </div>
          <div className="space-y-1">
            <p className="text-zinc-500 text-xs uppercase tracking-wider">Platform</p>
            <p className="text-zinc-200">Windows (Tauri v2)</p>
          </div>
          <div className="space-y-1">
            <p className="text-zinc-500 text-xs uppercase tracking-wider">Runtime</p>
            <p className="text-zinc-200">Rust + React</p>
          </div>
          <div className="space-y-1">
            <p className="text-zinc-500 text-xs uppercase tracking-wider">IPC</p>
            <p className="text-zinc-200">Windows Named Pipes</p>
          </div>
        </div>
      </div>

      <div className="space-y-3">
        <h3 className="text-sm font-medium text-zinc-300">Architecture</h3>
        <div className="space-y-2 text-sm text-zinc-400">
          {[
            { label: 'bat-shell', desc: 'Tauri desktop shell and React frontend' },
            { label: 'bat-gateway', desc: 'Session management, SQLite storage, event bus' },
            { label: 'bat-agent', desc: 'Anthropic API client, tool executor' },
            { label: 'bat-types', desc: 'Shared types and IPC protocol definitions' },
          ].map(({ label, desc }) => (
            <div key={label} className="flex gap-3">
              <span className="font-mono text-[#39FF14] flex-shrink-0 w-24">{label}</span>
              <span>{desc}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="border-t border-zinc-700 pt-5">
        <p className="text-xs text-zinc-600">
          Config and data stored in <span className="font-mono text-zinc-500">~/.batchismo/</span>
        </p>
      </div>
    </div>
  )
}
