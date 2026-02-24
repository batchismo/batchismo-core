import { useState } from 'react'
import type { ToolCall, ToolResult } from '../types'
import { TOOL_DISPLAY } from '../types'

interface Props {
  toolCall: ToolCall
  result?: ToolResult
}

export function ToolCallBlock({ toolCall, result }: Props) {
  const [open, setOpen] = useState(false)
  return (
    <div className="my-1 rounded border border-zinc-700 bg-zinc-900 text-xs">
      <button
        onClick={() => setOpen(o => !o)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left text-zinc-400 hover:text-zinc-200"
      >
        <span className="text-[#39FF14]">
          {TOOL_DISPLAY[toolCall.name]?.icon ?? '🔧'}{' '}
          {TOOL_DISPLAY[toolCall.name]?.name ?? toolCall.name}
        </span>
        <span className="ml-auto">{open ? '▲' : '▼'}</span>
      </button>
      {open && (
        <div className="border-t border-zinc-700 px-3 py-2 space-y-2">
          <div>
            <div className="text-zinc-500 mb-1">Input</div>
            <pre className="whitespace-pre-wrap break-all text-zinc-300">
              {JSON.stringify(toolCall.input, null, 2)}
            </pre>
          </div>
          {result && (
            <div>
              <div className={`mb-1 ${result.is_error ? 'text-red-400' : 'text-zinc-500'}`}>
                {result.is_error ? 'Error' : 'Result'}
              </div>
              <pre className={`whitespace-pre-wrap break-all ${result.is_error ? 'text-red-300' : 'text-zinc-300'}`}>
                {result.content}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
