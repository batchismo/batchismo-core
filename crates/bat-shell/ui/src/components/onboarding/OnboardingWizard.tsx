import { useState } from 'react'
import { completeOnboarding } from '../../lib/tauri'
import { WelcomeStep } from './WelcomeStep'
import { ApiKeyStep } from './ApiKeyStep'
import { OpenAIKeyStep } from './OpenAIKeyStep'
import { NameStep } from './NameStep'
import { AccessStep } from './AccessStep'
import { ReadyStep } from './ReadyStep'

const STEPS = ['welcome', 'apikey', 'openaikey', 'name', 'access', 'ready'] as const
type Step = typeof STEPS[number]

interface Props {
  onComplete: () => void
}

export function OnboardingWizard({ onComplete }: Props) {
  const [step, setStep] = useState<Step>('welcome')
  const [apiKey, setApiKey] = useState('')
  const [openaiApiKey, setOpenaiApiKey] = useState('')
  const [agentName, setAgentName] = useState('')
  const [folders, setFolders] = useState<[string, string, boolean][]>([])
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')

  const stepIndex = STEPS.indexOf(step)

  function next() {
    const i = STEPS.indexOf(step)
    if (i < STEPS.length - 1) setStep(STEPS[i + 1])
  }

  function back() {
    const i = STEPS.indexOf(step)
    if (i > 0) setStep(STEPS[i - 1])
  }

  async function finish() {
    setSaving(true)
    setError('')
    try {
      await completeOnboarding(
        agentName,
        apiKey,
        openaiApiKey.trim() || null,
        folders,
      )
      onComplete()
    } catch (e) {
      setError(String(e))
      setSaving(false)
    }
  }

  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100 items-center justify-center">
      <div className="w-full max-w-lg mx-auto px-6">
        {/* Progress dots */}
        <div className="flex justify-center gap-2 mb-8">
          {STEPS.map((s, i) => (
            <div
              key={s}
              className={`w-2 h-2 rounded-full transition-colors ${
                i <= stepIndex ? 'bg-indigo-500' : 'bg-zinc-700'
              }`}
            />
          ))}
        </div>

        {/* Step content */}
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-8 shadow-2xl">
          {step === 'welcome' && <WelcomeStep onNext={next} />}
          {step === 'apikey' && (
            <ApiKeyStep
              apiKey={apiKey}
              setApiKey={setApiKey}
              onNext={next}
              onBack={back}
            />
          )}
          {step === 'openaikey' && (
            <OpenAIKeyStep
              apiKey={openaiApiKey}
              setApiKey={setOpenaiApiKey}
              onNext={next}
              onBack={back}
            />
          )}
          {step === 'name' && (
            <NameStep
              name={agentName}
              setName={setAgentName}
              onNext={next}
              onBack={back}
            />
          )}
          {step === 'access' && (
            <AccessStep
              folders={folders}
              setFolders={setFolders}
              onNext={next}
              onBack={back}
            />
          )}
          {step === 'ready' && (
            <ReadyStep
              name={agentName}
              onFinish={finish}
              onBack={back}
              saving={saving}
              error={error}
            />
          )}
        </div>
      </div>
    </div>
  )
}
