import { useState } from 'react'

interface Props {
  onNext: () => void
  onBack: () => void
  onSkip: () => void
}

export function ChannelStep({ onNext, onBack, onSkip }: Props) {
  const [selectedChannel, setSelectedChannel] = useState<'telegram' | 'discord' | null>(null)
  const [telegramToken, setTelegramToken] = useState('')
  const [discordToken, setDiscordToken] = useState('')
  const [showToken, setShowToken] = useState(false)

  const handleNext = () => {
    // For now, we don't actually save the channel config during onboarding
    // This is just a placeholder step to match the spec
    // In a full implementation, you'd save these to the config
    onNext()
  }

  return (
    <div>
      <h2 className="text-xl font-bold text-white mb-2">Connect a Channel (Optional)</h2>
      <p className="text-zinc-400 text-sm mb-2">
        Want to chat with your agent from your phone or Discord? Set up a connection below.
      </p>
      <p className="text-zinc-500 text-xs mb-6">
        You can skip this step and use the built-in chat only, or set this up later in Settings.
      </p>

      <div className="space-y-3 mb-6">
        {/* Telegram */}
        <div 
          className={`border rounded-xl p-4 cursor-pointer transition-colors ${
            selectedChannel === 'telegram' 
              ? 'border-[#39FF14] bg-[#39FF14]/5' 
              : 'border-zinc-700 hover:border-zinc-600'
          }`}
          onClick={() => setSelectedChannel(selectedChannel === 'telegram' ? null : 'telegram')}
        >
          <div className="flex items-center gap-3 mb-2">
            <input 
              type="radio" 
              checked={selectedChannel === 'telegram'}
              onChange={() => setSelectedChannel(selectedChannel === 'telegram' ? null : 'telegram')}
              className="w-4 h-4" 
            />
            <h3 className="font-medium text-white flex items-center gap-2">
              <span className="text-blue-400">📱</span>
              Telegram
            </h3>
          </div>
          <p className="text-sm text-zinc-400 mb-3">
            Chat with your agent through a Telegram bot. Requires creating a bot via @BotFather.
          </p>
          
          {selectedChannel === 'telegram' && (
            <div className="space-y-3" onClick={e => e.stopPropagation()}>
              <div className="bg-zinc-800/50 border border-zinc-700 rounded-lg p-3">
                <h4 className="text-sm font-medium text-white mb-2">Setup Instructions:</h4>
                <ol className="text-xs text-zinc-400 space-y-1">
                  <li>1. Message @BotFather on Telegram</li>
                  <li>2. Send /newbot and follow the prompts</li>
                  <li>3. Copy your bot token and paste it below</li>
                </ol>
              </div>
              <div className="relative">
                <input
                  type={showToken ? 'text' : 'password'}
                  value={telegramToken}
                  onChange={e => setTelegramToken(e.target.value)}
                  placeholder="1234567890:ABCDEF..."
                  className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-[#39FF14] font-mono"
                />
                <button
                  onClick={() => setShowToken(!showToken)}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300 text-xs"
                >
                  {showToken ? 'Hide' : 'Show'}
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Discord */}
        <div 
          className={`border rounded-xl p-4 cursor-pointer transition-colors ${
            selectedChannel === 'discord' 
              ? 'border-[#39FF14] bg-[#39FF14]/5' 
              : 'border-zinc-700 hover:border-zinc-600'
          }`}
          onClick={() => setSelectedChannel(selectedChannel === 'discord' ? null : 'discord')}
        >
          <div className="flex items-center gap-3 mb-2">
            <input 
              type="radio" 
              checked={selectedChannel === 'discord'}
              onChange={() => setSelectedChannel(selectedChannel === 'discord' ? null : 'discord')}
              className="w-4 h-4" 
            />
            <h3 className="font-medium text-white flex items-center gap-2">
              <span className="text-indigo-400">💬</span>
              Discord
            </h3>
          </div>
          <p className="text-sm text-zinc-400 mb-3">
            Add your agent to a Discord server. Requires creating an application on Discord Developer Portal.
          </p>
          
          {selectedChannel === 'discord' && (
            <div className="space-y-3" onClick={e => e.stopPropagation()}>
              <div className="bg-zinc-800/50 border border-zinc-700 rounded-lg p-3">
                <h4 className="text-sm font-medium text-white mb-2">Setup Instructions:</h4>
                <ol className="text-xs text-zinc-400 space-y-1">
                  <li>1. Go to discord.com/developers/applications</li>
                  <li>2. Create a new application → Bot section</li>
                  <li>3. Copy your bot token and paste it below</li>
                </ol>
              </div>
              <div className="relative">
                <input
                  type={showToken ? 'text' : 'password'}
                  value={discordToken}
                  onChange={e => setDiscordToken(e.target.value)}
                  placeholder="MTAxNjA..."
                  className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-[#39FF14] font-mono"
                />
                <button
                  onClick={() => setShowToken(!showToken)}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300 text-xs"
                >
                  {showToken ? 'Hide' : 'Show'}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="flex justify-between mt-8">
        <button
          onClick={onBack}
          className="px-4 py-2 text-zinc-400 hover:text-zinc-200 text-sm transition-colors"
        >
          ← Back
        </button>
        <div className="flex gap-2">
          <button
            onClick={onSkip}
            className="px-4 py-2 text-zinc-400 hover:text-zinc-200 text-sm transition-colors"
          >
            Skip
          </button>
          <button
            onClick={handleNext}
            disabled={selectedChannel !== null && (
              (selectedChannel === 'telegram' && !telegramToken.trim()) ||
              (selectedChannel === 'discord' && !discordToken.trim())
            )}
            className="px-6 py-2.5 bg-[#39FF14] hover:bg-[#2bcc10] disabled:opacity-40 disabled:cursor-not-allowed text-black font-medium rounded-lg transition-colors"
          >
            Next →
          </button>
        </div>
      </div>
    </div>
  )
}