import { useEffect, useState } from 'react'
import type { SessionMeta, AppView } from './types'
import { useChat } from './hooks/useChat'
import { getSession, isOnboardingComplete } from './lib/tauri'
import { Sidebar } from './components/Sidebar'
import { ChatPanel } from './components/ChatPanel'
import { InputBar } from './components/InputBar'
import { StatusBar } from './components/StatusBar'
import { SettingsPanel } from './components/settings/SettingsPanel'
import { LogsPanel } from './components/LogsPanel'
import { OnboardingWizard } from './components/onboarding/OnboardingWizard'

export default function App() {
  const { messages, streamingText, agentStatus, error, send } = useChat()
  const [session, setSession] = useState<SessionMeta | null>(null)
  const [activeView, setActiveView] = useState<AppView>('chat')
  const [onboarded, setOnboarded] = useState<boolean | null>(null) // null = loading

  useEffect(() => {
    isOnboardingComplete().then(setOnboarded).catch(() => setOnboarded(true))
    getSession().then(setSession).catch(console.error)
  }, [])

  // Refresh session (token counts) after each turn completes
  useEffect(() => {
    if (agentStatus === 'idle') {
      getSession().then(setSession).catch(console.error)
    }
  }, [agentStatus])

  // Loading state
  if (onboarded === null) {
    return <div className="flex h-screen bg-zinc-950 items-center justify-center text-zinc-500">Loading...</div>
  }

  // Onboarding wizard
  if (!onboarded) {
    return (
      <OnboardingWizard
        onComplete={() => {
          setOnboarded(true)
          getSession().then(setSession).catch(console.error)
        }}
      />
    )
  }

  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100 overflow-hidden">
      {/* Sidebar navigation */}
      <Sidebar activeView={activeView} onNavigate={setActiveView} />

      {/* Main content area */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* Header */}
        <header className="flex items-center gap-3 border-b border-zinc-800 bg-zinc-900 px-4 py-2.5 flex-shrink-0">
          <span className="text-base font-semibold tracking-tight text-white">
            {activeView === 'chat' ? 'Chat' : activeView === 'logs' ? 'Audit Log' : 'Settings'}
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
        ) : activeView === 'logs' ? (
          <div className="flex-1 overflow-hidden">
            <LogsPanel />
          </div>
        ) : (
          <div className="flex-1 overflow-hidden">
            <SettingsPanel />
          </div>
        )}
      </div>
    </div>
  )
}
