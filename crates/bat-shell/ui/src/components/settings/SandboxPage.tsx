import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { BatConfig } from '../../types'

export function SandboxPage() {
  const [memoryLimitMb, setMemoryLimitMb] = useState(512)
  const [maxConcurrent, setMaxConcurrent] = useState(5)
  const [timeoutMinutes, setTimeoutMinutes] = useState(60)
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    invoke<BatConfig>('get_config').then(cfg => {
      setMemoryLimitMb(cfg.sandbox.memory_limit_mb)
      setMaxConcurrent(cfg.sandbox.max_concurrent_subagents)
      setTimeoutMinutes(cfg.sandbox.subagent_timeout_minutes ?? 60)
    })
  }, [])

  const save = async () => {
    try {
      const cfg = await invoke<BatConfig>('get_config')
      cfg.sandbox.memory_limit_mb = memoryLimitMb
      cfg.sandbox.max_concurrent_subagents = maxConcurrent
      cfg.sandbox.subagent_timeout_minutes = timeoutMinutes
      await invoke('update_config', { config: cfg })
      setSaved(true)
      setError('')
      setTimeout(() => setSaved(false), 2000)
    } catch (e) {
      setError(String(e))
    }
  }

  return (
    <div className="max-w-lg">
      <h2 className="text-lg font-semibold text-white mb-1">Sandbox &amp; Security</h2>
      <p className="text-sm text-zinc-400 mb-6">
        Resource limits for subagent processes and sandboxing.
      </p>

      <div className="space-y-5">
        <div>
          <label className="block text-sm font-medium text-zinc-300 mb-1">
            Memory Limit (MB)
          </label>
          <input
            type="number"
            min={64}
            max={8192}
            value={memoryLimitMb}
            onChange={e => setMemoryLimitMb(Number(e.target.value))}
            className="w-full bg-zinc-800 border border-zinc-700 rounded-md px-3 py-2 text-white text-sm focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
          <p className="text-xs text-zinc-500 mt-1">Maximum memory per subagent process.</p>
        </div>

        <div>
          <label className="block text-sm font-medium text-zinc-300 mb-1">
            Max Concurrent Subagents
          </label>
          <input
            type="number"
            min={1}
            max={20}
            value={maxConcurrent}
            onChange={e => setMaxConcurrent(Number(e.target.value))}
            className="w-full bg-zinc-800 border border-zinc-700 rounded-md px-3 py-2 text-white text-sm focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
          <p className="text-xs text-zinc-500 mt-1">How many subagents can run at the same time.</p>
        </div>

        <div>
          <label className="block text-sm font-medium text-zinc-300 mb-1">
            Subagent Timeout (minutes)
          </label>
          <input
            type="number"
            min={5}
            max={1440}
            value={timeoutMinutes}
            onChange={e => setTimeoutMinutes(Number(e.target.value))}
            className="w-full bg-zinc-800 border border-zinc-700 rounded-md px-3 py-2 text-white text-sm focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
          <p className="text-xs text-zinc-500 mt-1">Auto-terminate subagents running longer than this.</p>
        </div>
      </div>

      <div className="mt-6 flex items-center gap-3">
        <button
          onClick={save}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-md transition-colors"
        >
          Save
        </button>
        {saved && <span className="text-sm text-green-400">✓ Saved</span>}
        {error && <span className="text-sm text-red-400">{error}</span>}
      </div>
    </div>
  )
}
