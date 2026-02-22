import { open } from '@tauri-apps/plugin-dialog'

interface Props {
  folders: [string, string, boolean][]
  setFolders: (folders: [string, string, boolean][]) => void
  onNext: () => void
  onBack: () => void
}

export function AccessStep({ folders, setFolders, onNext, onBack }: Props) {
  async function addFolder() {
    const selected = await open({ directory: true, multiple: false })
    if (selected && typeof selected === 'string') {
      // Don't add duplicates
      if (!folders.some(([p]) => p === selected)) {
        setFolders([...folders, [selected, 'read-write', true]])
      }
    }
  }

  function removeFolder(index: number) {
    setFolders(folders.filter((_, i) => i !== index))
  }

  function toggleAccess(index: number) {
    const updated = [...folders]
    const current = updated[index][1]
    updated[index] = [
      updated[index][0],
      current === 'read-write' ? 'read-only' : 'read-write',
      updated[index][2],
    ]
    setFolders(updated)
  }

  return (
    <div>
      <h2 className="text-xl font-bold text-white mb-2">Set Up Access</h2>
      <p className="text-zinc-400 text-sm mb-2">
        Choose which folders your agent can work with. It won't be able to touch anything else.
      </p>
      <p className="text-zinc-500 text-xs mb-6">
        You can always change this later in Settings → Path Policies.
      </p>

      {/* Folder list */}
      <div className="space-y-2 mb-4 max-h-48 overflow-y-auto">
        {folders.map(([path, access], i) => (
          <div key={path} className="flex items-center gap-2 bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2">
            <div className="flex-1 min-w-0">
              <p className="text-sm text-zinc-200 truncate font-mono" title={path}>{path}</p>
            </div>
            <button
              onClick={() => toggleAccess(i)}
              className={`px-2 py-0.5 text-xs rounded border transition-colors whitespace-nowrap ${
                access === 'read-write'
                  ? 'border-emerald-600/50 bg-emerald-900/20 text-emerald-400'
                  : 'border-amber-600/50 bg-amber-900/20 text-amber-400'
              }`}
            >
              {access === 'read-write' ? 'Read/Write' : 'Read Only'}
            </button>
            <button
              onClick={() => removeFolder(i)}
              className="text-zinc-500 hover:text-red-400 transition-colors text-sm"
              title="Remove"
            >
              ✕
            </button>
          </div>
        ))}
      </div>

      <button
        onClick={addFolder}
        className="w-full py-2 border border-dashed border-zinc-700 rounded-lg text-zinc-400 hover:text-zinc-300 hover:border-zinc-600 transition-colors text-sm"
      >
        + Add Folder
      </button>

      <div className="flex justify-between mt-8">
        <button
          onClick={onBack}
          className="px-4 py-2 text-zinc-400 hover:text-zinc-200 text-sm transition-colors"
        >
          ← Back
        </button>
        <button
          onClick={onNext}
          disabled={folders.length === 0}
          className="px-6 py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors"
        >
          Next →
        </button>
      </div>
    </div>
  )
}
