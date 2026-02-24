import { useState, useEffect } from 'react'
import type { BatConfig } from '../../types'
import { getConfig, updateConfig } from '../../lib/tauri'

export function ChannelsPage() {
  const [config, setConfig] = useState<BatConfig | null>(null)
  const [tgEnabled, setTgEnabled] = useState(false)
  const [tgToken, setTgToken] = useState('')
  const [tgAllowFrom, setTgAllowFrom] = useState('')
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    getConfig().then(cfg => {
      setConfig(cfg)
      const tg = cfg.channels?.telegram
      if (tg) {
        setTgEnabled(tg.enabled)
        setTgToken(tg.bot_token || '')
        setTgAllowFrom((tg.allow_from || []).join(', '))
      }
    })
  }, [])

  const handleSave = async () => {
    if (!config) return
    const allowFrom = tgAllowFrom
      .split(',')
      .map(s => s.trim())
      .filter(s => s.length > 0)
      .map(s => parseInt(s, 10))
      .filter(n => !isNaN(n))

    const updated = {
      ...config,
      channels: {
        ...config.channels,
        telegram: {
          enabled: tgEnabled,
          bot_token: tgToken,
          allow_from: allowFrom,
        },
      },
    }
    await updateConfig(updated)
    setConfig(updated)
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  if (!config) return <div className="p-4 text-zinc-500">Loading...</div>

  return (
    <div className="p-4 space-y-6">
      {/* Telegram */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-semibold text-zinc-200">Telegram</h3>
            <p className="text-xs text-zinc-500 mt-0.5">Connect a Telegram bot to chat from your phone</p>
          </div>
          <button
            onClick={() => setTgEnabled(!tgEnabled)}
            className={`relative w-10 h-5 rounded-full transition-colors ${
              tgEnabled ? 'bg-[#39FF14]' : 'bg-zinc-700'
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                tgEnabled ? 'translate-x-5' : ''
              }`}
            />
          </button>
        </div>

        {tgEnabled && (
          <div className="space-y-3 pl-0.5">
            <div>
              <label className="text-xs text-zinc-400 font-medium">Bot Token</label>
              <input
                type="password"
                value={tgToken}
                onChange={e => setTgToken(e.target.value)}
                placeholder="123456:ABC-DEF..."
                className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white
                           placeholder-zinc-600 outline-none focus:border-[#39FF14]"
              />
              <p className="text-[10px] text-zinc-600 mt-1">
                Get one from <a href="https://t.me/BotFather" className="text-[#39FF14] hover:underline" target="_blank">@BotFather</a> on Telegram
              </p>
            </div>
            <div>
              <label className="text-xs text-zinc-400 font-medium">Allowed User IDs</label>
              <input
                value={tgAllowFrom}
                onChange={e => setTgAllowFrom(e.target.value)}
                placeholder="123456789, 987654321"
                className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white
                           placeholder-zinc-600 outline-none focus:border-[#39FF14]"
              />
              <p className="text-[10px] text-zinc-600 mt-1">
                Comma-separated Telegram user IDs. Leave empty to allow anyone (not recommended).
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Discord placeholder */}
      <div className="flex items-center justify-between opacity-50">
        <div>
          <h3 className="text-sm font-semibold text-zinc-200">Discord</h3>
          <p className="text-xs text-zinc-500 mt-0.5">Coming in a future release</p>
        </div>
        <span className="text-xs text-zinc-600 bg-zinc-800 px-2 py-0.5 rounded">Soon</span>
      </div>

      {/* Save */}
      <button
        onClick={handleSave}
        className={`px-4 py-1.5 rounded text-sm font-medium transition-colors ${
          saved
            ? 'bg-emerald-600 text-white'
            : 'bg-[#39FF14] hover:bg-[#2bcc10] text-black'
        }`}
      >
        {saved ? '✓ Saved' : 'Save Changes'}
      </button>
      <p className="text-[10px] text-zinc-600">Restart the app after changing channel settings.</p>
    </div>
  )
}
