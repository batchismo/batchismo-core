import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface Skill {
  name: string
  path: string
  content: string
  enabled: boolean
  tools: SkillTool[]
  lastModified: string
}

interface SkillTool {
  name: string
  description: string
  command: string
  args?: string[]
  inputSchema?: object
  workingDir?: string
}

export function SkillsPage() {
  const [skills, setSkills] = useState<Skill[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [selectedSkill, setSelectedSkill] = useState<string | null>(null)

  // Load skills on component mount
  useEffect(() => {
    loadSkills()
  }, [])

  const loadSkills = async () => {
    try {
      setLoading(true)
      setError(null)
      const skillList = await invoke<Skill[]>('list_skills')
      setSkills(skillList)
    } catch (err) {
      console.error('Failed to load skills:', err)
      setError('Failed to load skills')
    } finally {
      setLoading(false)
    }
  }

  const toggleSkill = async (skillName: string, enabled: boolean) => {
    try {
      await invoke('set_skill_enabled', { name: skillName, enabled })
      setSkills(skills.map(skill => 
        skill.name === skillName 
          ? { ...skill, enabled }
          : skill
      ))
    } catch (err) {
      console.error('Failed to toggle skill:', err)
      setError('Failed to update skill')
    }
  }

  const formatLastModified = (lastModified: string) => {
    try {
      const date = new Date(lastModified)
      return date.toLocaleString()
    } catch {
      return 'Unknown'
    }
  }

  const getSelectedSkillDetails = () => {
    if (!selectedSkill) return null
    return skills.find(skill => skill.name === selectedSkill)
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="bg-red-950 border border-red-800 rounded-md p-4">
        <div className="flex">
          <div className="ml-3">
            <h3 className="text-sm font-medium text-red-200">Error</h3>
            <div className="mt-2 text-sm text-red-300">{error}</div>
          </div>
        </div>
        <button
          onClick={loadSkills}
          className="mt-3 bg-red-800 hover:bg-red-700 text-white px-3 py-1 rounded text-sm"
        >
          Retry
        </button>
      </div>
    )
  }

  const selectedSkillData = getSelectedSkillDetails()

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-xl font-semibold text-white mb-2">Skills</h2>
        <p className="text-zinc-400 text-sm">
          Skills provide contextual knowledge and capabilities to your agent. Each skill contains guidance
          on when and how to use specific tools and approaches.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Skills List */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-medium text-white">Available Skills</h3>
            <button
              onClick={loadSkills}
              className="text-zinc-400 hover:text-white p-1 rounded"
              title="Refresh skills"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            </button>
          </div>

          {skills.length === 0 ? (
            <div className="bg-zinc-800 rounded-lg p-4 text-center">
              <div className="text-zinc-500 mb-2">📄</div>
              <p className="text-zinc-400 text-sm">No skills found</p>
              <p className="text-zinc-500 text-xs mt-1">
                Add skills to ~/.batchismo/workspace/skills/
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {skills.map((skill) => (
                <div
                  key={skill.name}
                  className={`bg-zinc-800 rounded-lg p-4 border transition-colors cursor-pointer ${
                    selectedSkill === skill.name
                      ? 'border-blue-500 bg-zinc-800'
                      : 'border-zinc-700 hover:border-zinc-600'
                  }`}
                  onClick={() => setSelectedSkill(skill.name)}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <h4 className="font-medium text-white">{skill.name}</h4>
                        {skill.tools.length > 0 && (
                          <span className="bg-blue-900 text-blue-300 px-2 py-1 rounded text-xs">
                            {skill.tools.length} tool{skill.tools.length !== 1 ? 's' : ''}
                          </span>
                        )}
                      </div>
                      <p className="text-zinc-400 text-sm mt-1">
                        Modified: {formatLastModified(skill.lastModified)}
                      </p>
                    </div>
                    <div className="flex items-center">
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          toggleSkill(skill.name, !skill.enabled)
                        }}
                        className={`relative inline-flex items-center h-6 rounded-full w-11 transition-colors ${
                          skill.enabled ? 'bg-blue-600' : 'bg-zinc-600'
                        }`}
                      >
                        <span
                          className={`inline-block w-4 h-4 bg-white rounded-full transition-transform ${
                            skill.enabled ? 'translate-x-6' : 'translate-x-1'
                          }`}
                        />
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Skill Details */}
        <div className="space-y-4">
          <h3 className="text-lg font-medium text-white">Skill Details</h3>
          
          {!selectedSkillData ? (
            <div className="bg-zinc-800 rounded-lg p-6 text-center">
              <p className="text-zinc-400">Select a skill to view its details</p>
            </div>
          ) : (
            <div className="bg-zinc-800 rounded-lg p-4 space-y-4">
              <div className="flex items-center justify-between">
                <h4 className="font-medium text-white text-lg">{selectedSkillData.name}</h4>
                <div className={`px-2 py-1 rounded text-xs ${
                  selectedSkillData.enabled 
                    ? 'bg-green-900 text-green-300' 
                    : 'bg-zinc-700 text-zinc-400'
                }`}>
                  {selectedSkillData.enabled ? 'Enabled' : 'Disabled'}
                </div>
              </div>

              <div>
                <h5 className="text-sm font-medium text-zinc-300 mb-2">Path</h5>
                <p className="text-zinc-400 text-sm font-mono bg-zinc-900 px-2 py-1 rounded">
                  {selectedSkillData.path}
                </p>
              </div>

              {selectedSkillData.tools.length > 0 && (
                <div>
                  <h5 className="text-sm font-medium text-zinc-300 mb-2">Tools</h5>
                  <div className="space-y-2">
                    {selectedSkillData.tools.map((tool, index) => (
                      <div key={index} className="bg-zinc-900 p-3 rounded">
                        <div className="font-mono text-sm text-blue-300">{tool.name}</div>
                        <div className="text-xs text-zinc-400 mt-1">{tool.description}</div>
                        <div className="text-xs text-zinc-500 mt-1 font-mono">
                          {tool.command} {tool.args?.join(' ') || ''}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <div>
                <h5 className="text-sm font-medium text-zinc-300 mb-2">Content Preview</h5>
                <div className="bg-zinc-900 p-3 rounded max-h-64 overflow-y-auto">
                  <pre className="text-xs text-zinc-300 whitespace-pre-wrap font-mono">
                    {selectedSkillData.content.substring(0, 1000)}
                    {selectedSkillData.content.length > 1000 && '...'}
                  </pre>
                </div>
              </div>

              <div className="pt-2 border-t border-zinc-700">
                <p className="text-xs text-zinc-500">
                  Skills are hot-reloaded when modified. Changes take effect immediately.
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}