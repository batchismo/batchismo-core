import { useEffect, useState } from 'react'
import type { MemoryFileInfo, ObservationSummary } from '../types'
import { getMemoryFiles, getMemoryFile, updateMemoryFile, getObservationSummary, triggerConsolidation } from '../lib/tauri'

export function MemoryPanel() {
  const [files, setFiles] = useState<MemoryFileInfo[]>([])
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [content, setContent] = useState('')
  const [editMode, setEditMode] = useState(false)
  const [editContent, setEditContent] = useState('')
  const [saving, setSaving] = useState(false)
  const [summary, setSummary] = useState<ObservationSummary | null>(null)
  const [consolidating, setConsolidating] = useState(false)
  const [consolidateResult, setConsolidateResult] = useState('')

  useEffect(() => {
    loadFiles()
    getObservationSummary().then(setSummary).catch(console.error)
  }, [])

  async function loadFiles() {
    try {
      const f = await getMemoryFiles()
      setFiles(f)
      if (f.length > 0 && !selectedFile) {
        selectFile(f[0].name)
      }
    } catch (e) {
      console.error('Failed to load memory files:', e)
    }
  }

  async function selectFile(name: string) {
    setSelectedFile(name)
    setEditMode(false)
    try {
      const c = await getMemoryFile(name)
      setContent(c)
      setEditContent(c)
    } catch (e) {
      setContent(`Error loading ${name}: ${e}`)
    }
  }

  async function handleSave() {
    if (!selectedFile) return
    setSaving(true)
    try {
      await updateMemoryFile(selectedFile, editContent)
      setContent(editContent)
      setEditMode(false)
    } catch (e) {
      console.error('Failed to save:', e)
    } finally {
      setSaving(false)
    }
  }

  async function handleConsolidate() {
    setConsolidating(true)
    setConsolidateResult('')
    try {
      const result = await triggerConsolidation()
      setConsolidateResult(result)
      // Reload files after consolidation
      await loadFiles()
      if (selectedFile) await selectFile(selectedFile)
      getObservationSummary().then(setSummary).catch(console.error)
    } catch (e) {
      setConsolidateResult(`Error: ${e}`)
    } finally {
      setConsolidating(false)
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`
    return `${(bytes / 1024).toFixed(1)} KB`
  }

  function formatTime(ts: string | null): string {
    if (!ts) return '—'
    try {
      return new Date(ts).toLocaleDateString('en-US', {
        month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
      })
    } catch { return ts }
  }

  return (
    <div className="flex h-full">
      {/* Left panel — file list + stats */}
      <div className="w-64 flex-shrink-0 border-r border-zinc-800 flex flex-col">
        {/* File list */}
        <div className="flex-1 overflow-y-auto">
          <div className="px-3 py-2 text-xs text-zinc-500 uppercase tracking-wider font-medium">
            Workspace Files
          </div>
          {files.map(file => (
            <button
              key={file.name}
              onClick={() => selectFile(file.name)}
              className={`w-full text-left px-3 py-2 text-sm transition-colors ${
                selectedFile === file.name
                  ? 'bg-zinc-800 text-white'
                  : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'
              }`}
            >
              <div className="font-medium truncate">{file.name}</div>
              <div className="text-xs text-zinc-600 mt-0.5">
                {formatBytes(file.sizeBytes)} · {formatTime(file.modifiedAt)}
              </div>
            </button>
          ))}
        </div>

        {/* Observation stats */}
        {summary && (
          <div className="border-t border-zinc-800 px-3 py-3">
            <div className="text-xs text-zinc-500 uppercase tracking-wider font-medium mb-2">
              Observations
            </div>
            <div className="space-y-1 text-xs">
              <div className="flex justify-between text-zinc-400">
                <span>Total</span>
                <span className="text-zinc-300 tabular-nums">{summary.totalObservations}</span>
              </div>
              <div className="flex justify-between text-zinc-400">
                <span>Sessions</span>
                <span className="text-zinc-300 tabular-nums">{summary.totalSessions}</span>
              </div>
              {summary.topTools.length > 0 && (
                <>
                  <div className="text-zinc-500 mt-2 mb-1">Top Tools</div>
                  {summary.topTools.slice(0, 3).map(([tool, count]) => (
                    <div key={tool} className="flex justify-between text-zinc-400">
                      <span className="text-emerald-400/70">{tool}</span>
                      <span className="tabular-nums">{count}×</span>
                    </div>
                  ))}
                </>
              )}
              {summary.topPaths.length > 0 && (
                <>
                  <div className="text-zinc-500 mt-2 mb-1">Top Paths</div>
                  {summary.topPaths.slice(0, 3).map(([path, count]) => (
                    <div key={path} className="flex justify-between text-zinc-400">
                      <span className="truncate text-cyan-400/70 mr-2" title={path}>
                        {path.split(/[/\\]/).pop()}
                      </span>
                      <span className="tabular-nums flex-shrink-0">{count}×</span>
                    </div>
                  ))}
                </>
              )}
            </div>

            {/* Consolidate button */}
            <button
              onClick={handleConsolidate}
              disabled={consolidating || (summary?.totalObservations ?? 0) === 0}
              className="w-full mt-3 px-3 py-1.5 text-xs bg-[#39FF14]/20 border border-[#39FF14]/30 text-[#39FF14] hover:bg-[#39FF14]/30 disabled:opacity-40 disabled:cursor-not-allowed rounded transition-colors"
            >
              {consolidating ? 'Consolidating...' : '🧠 Consolidate Now'}
            </button>
            {consolidateResult && (
              <p className={`text-xs mt-1.5 ${consolidateResult.startsWith('Error') ? 'text-red-400' : 'text-zinc-400'}`}>
                {consolidateResult}
              </p>
            )}
          </div>
        )}
      </div>

      {/* Right panel — file content */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Toolbar */}
        {selectedFile && (
          <div className="flex items-center gap-2 px-4 py-2 border-b border-zinc-800 bg-zinc-900/50 flex-shrink-0">
            <span className="text-sm font-medium text-zinc-300">{selectedFile}</span>
            <div className="flex-1" />
            {editMode ? (
              <>
                <button
                  onClick={() => { setEditMode(false); setEditContent(content) }}
                  className="px-3 py-1 text-xs text-zinc-400 hover:text-zinc-200 rounded"
                >
                  Cancel
                </button>
                <button
                  onClick={handleSave}
                  disabled={saving}
                  className="px-3 py-1 text-xs bg-[#39FF14] hover:bg-[#2bcc10] disabled:opacity-50 text-black rounded"
                >
                  {saving ? 'Saving...' : 'Save'}
                </button>
              </>
            ) : (
              <button
                onClick={() => setEditMode(true)}
                className="px-3 py-1 text-xs text-zinc-400 hover:text-zinc-200 border border-zinc-700 rounded"
              >
                Edit
              </button>
            )}
          </div>
        )}

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-4">
          {!selectedFile ? (
            <div className="flex items-center justify-center h-full text-zinc-600">
              Select a file to view
            </div>
          ) : editMode ? (
            <textarea
              value={editContent}
              onChange={e => setEditContent(e.target.value)}
              className="w-full h-full bg-zinc-900 text-zinc-200 font-mono text-sm p-3 rounded-lg border border-zinc-700 focus:outline-none focus:border-[#39FF14] resize-none"
              spellCheck={false}
            />
          ) : (
            <pre className="text-sm text-zinc-300 font-mono whitespace-pre-wrap break-words leading-relaxed">
              {content}
            </pre>
          )}
        </div>
      </div>
    </div>
  )
}
