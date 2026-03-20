import { useState } from 'react'
import { sendMessage } from '../../lib/tauri'

interface Props {
  name: string
  folders: [string, string, boolean][]
  onFinish: () => void
  onBack: () => void
  saving: boolean
  error: string
}

export function FirstTaskStep({ name, folders, onFinish, onBack, saving, error }: Props) {
  const [selectedTask, setSelectedTask] = useState<string | null>(null)
  const [sendingTask, setSendingTask] = useState(false)

  // Generate suggested tasks based on the folders they granted access to
  function getSuggestedTasks(): { title: string; description: string; message: string }[] {
    const tasks = []
    
    if (folders.length > 0) {
      // Always suggest a basic file listing
      tasks.push({
        title: "Explore your files",
        description: `List what's in ${folders[0][0]}`,
        message: `Hi ${name}! Can you show me what files are in ${folders[0][0]}?`
      })
      
      // If they have multiple folders, suggest organizing
      if (folders.length > 1) {
        tasks.push({
          title: "Organize your workspace", 
          description: "Get an overview of all your accessible folders",
          message: `Hi ${name}! Can you give me an overview of all the folders I've given you access to? Show me what types of files are in each one.`
        })
      }
      
      // Look for common folder patterns and suggest specific tasks
      const hasDocuments = folders.some(([path]) => path.toLowerCase().includes('document'))
      const hasProjects = folders.some(([path]) => path.toLowerCase().includes('project'))
      const hasDesktop = folders.some(([path]) => path.toLowerCase().includes('desktop'))
      
      if (hasDocuments) {
        tasks.push({
          title: "Review your documents",
          description: "Find and summarize recent documents",
          message: `Hi ${name}! Can you look through my documents folder and tell me what types of files I have there? If you find any recent documents, can you give me a brief summary of what they contain?`
        })
      }
      
      if (hasProjects) {
        tasks.push({
          title: "Project overview",
          description: "Analyze your project folders",
          message: `Hi ${name}! Can you examine my projects folder and tell me about the different projects I'm working on? Look for README files, code, or documentation that might give you clues about what each project does.`
        })
      }
      
      if (hasDesktop) {
        tasks.push({
          title: "Clean up desktop",
          description: "Help organize files on your desktop",
          message: `Hi ${name}! Can you look at my desktop folder and suggest how I might better organize the files there? Group similar files and suggest folder structures if needed.`
        })
      }
    }
    
    // Fallback suggestions if no specific patterns detected
    if (tasks.length < 3) {
      tasks.push({
        title: "Get to know you",
        description: "Start with a simple conversation",
        message: `Hi ${name}! I just finished setting up Batchismo. Can you tell me a bit about what you can help me with?`
      })
    }
    
    return tasks.slice(0, 3) // Limit to 3 suggestions
  }

  const suggestedTasks = getSuggestedTasks()

  async function handleTryNow() {
    if (!selectedTask) return
    
    setSendingTask(true)
    try {
      await sendMessage(selectedTask)
      // Complete onboarding and go to chat
      onFinish()
    } catch (e) {
      console.error('Failed to send message:', e)
    } finally {
      setSendingTask(false)
    }
  }

  return (
    <div>
      <div className="text-center mb-6">
        <div className="w-14 h-14 rounded-full bg-emerald-600/20 border border-emerald-500/30 flex items-center justify-center mx-auto mb-4">
          <span className="text-2xl">🎯</span>
        </div>
        <h2 className="text-xl font-bold text-white mb-2">Ready to Go!</h2>
        <p className="text-zinc-400">
          <span className="text-[#39FF14] font-medium">{name}</span> is set up and ready to help.
          Here are some suggested first tasks based on the folders you've shared:
        </p>
      </div>

      <div className="space-y-3 mb-6">
        {suggestedTasks.map((task, index) => (
          <div
            key={index}
            className={`border rounded-xl p-4 cursor-pointer transition-colors ${
              selectedTask === task.message
                ? 'border-[#39FF14] bg-[#39FF14]/5'
                : 'border-zinc-700 hover:border-zinc-600'
            }`}
            onClick={() => setSelectedTask(task.message)}
          >
            <div className="flex items-start gap-3">
              <input
                type="radio"
                checked={selectedTask === task.message}
                onChange={() => setSelectedTask(task.message)}
                className="w-4 h-4 mt-0.5"
              />
              <div className="flex-1">
                <h3 className="font-medium text-white mb-1">{task.title}</h3>
                <p className="text-sm text-zinc-400 mb-2">{task.description}</p>
                <p className="text-xs text-zinc-500 font-mono bg-zinc-800/50 rounded p-2 border border-zinc-700">
                  "{task.message}"
                </p>
              </div>
            </div>
          </div>
        ))}
      </div>

      {error && (
        <p className="text-sm text-red-400 mb-4">
          Something went wrong: {error}
        </p>
      )}

      <div className="flex justify-between mt-8">
        <button
          onClick={onBack}
          disabled={saving || sendingTask}
          className="px-4 py-2 text-zinc-400 hover:text-zinc-200 text-sm transition-colors disabled:opacity-50"
        >
          ← Back
        </button>
        <div className="flex gap-2">
          <button
            onClick={onFinish}
            disabled={saving || sendingTask}
            className="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 text-sm rounded-lg transition-colors"
          >
            {saving ? 'Setting up...' : 'Start Chatting'}
          </button>
          <button
            onClick={handleTryNow}
            disabled={!selectedTask || saving || sendingTask}
            className="px-6 py-2.5 bg-[#39FF14] hover:bg-[#2bcc10] disabled:opacity-40 disabled:cursor-not-allowed text-black font-medium rounded-lg transition-colors"
          >
            {sendingTask ? 'Sending...' : 'Try It Now →'}
          </button>
        </div>
      </div>
    </div>
  )
}