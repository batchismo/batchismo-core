import { useEffect, useState } from 'react'
import type { SessionMeta, AppView } from './types'
import { useChat } from './hooks/useChat'
import { getSession } from './lib/tauri'
import { Sidebar } from './components/Sidebar'
import { ChatPanel } from './components/ChatPanel'
import { InputBar } from './components/InputBar'
import { StatusBar } from './components/StatusBar'
import { SettingsPanel } from './components/settings/SettingsPanel'

export default function App() {
  const { messages, streamingText, agentStatus, error, send } = useChat()
  const [session, setSession] = useState<SessionMeta | null>(null)
  const [activeView, setActiveView] = useState<AppView>('chat')

  useEffect(() => {
    getSession().then(setSession).catch(console.error)
  }, [])

  // Refresh session (token counts) after each turn completes
  useEffect(() => {
    if (agentStatus === 'idle') {
      getSession().then(setSession).catch(console.error)
    }
  }, [agentStatus])

  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100 overflow-hidden">
      {/* Sidebar navigation */}
      <Sidebar activeView={activeView} onNavigate={setActiveView} />

      {/* Main content area */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* Header */}
        <header className="flex items-center gap-3 border-b border-zinc-800 bg-zinc-900 px-4 py-2.5 flex-shrink-0">
          <span className="text-base font-semibold tracking-tight text-white">
            {activeView === 'chat' ? 'Chat' : 'Settings'}
          </span>
          {activeView === 'chat' && session && (
            <span className="text-xs text-zinc-500 font-mono ml-1">
              {session.model.replace('anthropic/', '')}
            </span>
          )}
        </header>

        {/* View content */}
        {activeView === 'chat' ? (
          <>
            <ChatPanel
              messages={messages}
              streamingText={streamingText}
              agentStatus={agentStatus}
            />
            <InputBar
              onSend={send}
              disabled={agentStatus !== 'idle'}
            />
            <StatusBar session={session} agentStatus={agentStatus} error={error} />
          </>
        ) : (
          <div className="flex-1 overflow-hidden">
            <SettingsPanel />
          </div>
        )}
      </div>
    </div>
  )
}
