import { useState, useEffect } from 'react'
import type { BatConfig } from '../../types'
import { getConfig, updateConfig } from '../../lib/tauri'

export function ChannelsPage() {
  const [config, setConfig] = useState<BatConfig | null>(null)
  const [tgEnabled, setTgEnabled] = useState(false)
  const [tgToken, setTgToken] = useState('')
  const [tgAllowFrom, setTgAllowFrom] = useState('')
  const [discordEnabled, setDiscordEnabled] = useState(false)
  const [discordToken, setDiscordToken] = useState('')
  const [discordAllowFrom, setDiscordAllowFrom] = useState('')
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
      const discord = cfg.channels?.discord
      if (discord) {
        setDiscordEnabled(discord.enabled)
        setDiscordToken(discord.bot_token || '')
        setDiscordAllowFrom((discord.allow_from || []).join(', '))
      }
    })
  }, [])

  const handleSave = async () => {
    if (!config) return
    
    const tgAllowFromNumbers = tgAllowFrom
      .split(',')
      .map(s => s.trim())
      .filter(s => s.length > 0)
      .map(s => parseInt(s, 10))
      .filter(n => !isNaN(n))

    const discordAllowFromNumbers = discordAllowFrom
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
          allow_from: tgAllowFromNumbers,
        },
        discord: {
          enabled: discordEnabled,
          bot_token: discordToken,
          allow_from: discordAllowFromNumbers,
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
                Comma-separated Telegram user IDs. Message <a href="https://t.me/userinfobot" className="text-[#39FF14] hover:underline" target="_blank">@userinfobot</a> on Telegram to get your ID. Leave empty to allow anyone (not recommended).
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Discord */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-semibold text-zinc-200">Discord</h3>
            <p className="text-xs text-zinc-500 mt-0.5">Connect a Discord bot for server/DM interactions</p>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-amber-400 bg-amber-900/30 px-2 py-0.5 rounded">Beta</span>
            <button
              onClick={() => setDiscordEnabled(!discordEnabled)}
              className={`relative w-10 h-5 rounded-full transition-colors ${
                discordEnabled ? 'bg-[#5865F2]' : 'bg-zinc-700'
              }`}
            >
              <span
                className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                  discordEnabled ? 'translate-x-5' : ''
                }`}
              />
            </button>
          </div>
        </div>

        {discordEnabled && (
          <div className="space-y-3 pl-0.5">
            <div>
              <label className="text-xs text-zinc-400 font-medium">Bot Token</label>
              <input
                type="password"
                value={discordToken}
                onChange={e => setDiscordToken(e.target.value)}
                placeholder="MTAxNjc4ODk..."
                className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white
                           placeholder-zinc-600 outline-none focus:border-[#5865F2]"
              />
              <p className="text-[10px] text-zinc-600 mt-1">
                Create a bot at <a href="https://discord.com/developers/applications" className="text-[#5865F2] hover:underline" target="_blank">Discord Developer Portal</a>
              </p>
            </div>
            <div>
              <label className="text-xs text-zinc-400 font-medium">Allowed User IDs</label>
              <input
                value={discordAllowFrom}
                onChange={e => setDiscordAllowFrom(e.target.value)}
                placeholder="123456789012345678, 876543210987654321"
                className="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm text-white
                           placeholder-zinc-600 outline-none focus:border-[#5865F2]"
              />
              <p className="text-[10px] text-zinc-600 mt-1">
                Comma-separated Discord user IDs. Enable Developer Mode in Discord, right-click a user, and copy ID. Leave empty to allow anyone (not recommended).
              </p>
            </div>
            <div className="p-3 bg-amber-900/20 border border-amber-600/30 rounded">
              <p className="text-xs text-amber-300">
                <strong>Note:</strong> Discord integration is currently a stub implementation. Full Discord support with the serenity crate will be added in a future release.
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Save */}
      <div className="space-y-2">
        <div className="flex items-center gap-2 p-2 bg-amber-500/10 border border-amber-500/20 rounded">
          <span className="text-amber-400 text-sm">⚠️</span>
          <p className="text-xs text-amber-300">Channel changes require an app restart to take effect.</p>
        </div>
        <button
          onClick={handleSave}
          className={`px-4 py-1.5 rounded text-sm font-medium transition-colors ${
            saved
              ? 'bg-emerald-600 text-white'
              : 'bg-[#39FF14] hover:bg-[#2bcc10] text-black'
          }`}
        >
          {saved ? '✓ Saved — Restart to apply' : 'Save Changes'}
        </button>
      </div>
    </div>
  )
}
