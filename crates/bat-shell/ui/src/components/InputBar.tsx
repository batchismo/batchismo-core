import { useState, useRef, KeyboardEvent, DragEvent, ClipboardEvent, useCallback } from 'react'
import type { ImageAttachment } from '../types'

interface Props {
  onSend: (content: string, images?: ImageAttachment[]) => void
  disabled: boolean
}

const ACCEPTED_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp']
const MAX_IMAGE_SIZE = 20 * 1024 * 1024 // 20MB

function fileToAttachment(file: File): Promise<ImageAttachment> {
  return new Promise((resolve, reject) => {
    if (!ACCEPTED_TYPES.includes(file.type)) {
      reject(new Error(`Unsupported image type: ${file.type}`))
      return
    }
    if (file.size > MAX_IMAGE_SIZE) {
      reject(new Error('Image too large (max 20MB)'))
      return
    }
    const reader = new FileReader()
    reader.onload = () => {
      const dataUrl = reader.result as string
      // Strip the "data:image/png;base64," prefix
      const base64 = dataUrl.split(',')[1]
      resolve({ data: base64, mediaType: file.type })
    }
    reader.onerror = () => reject(new Error('Failed to read image'))
    reader.readAsDataURL(file)
  })
}

export function InputBar({ onSend, disabled }: Props) {
  const [text, setText] = useState('')
  const [images, setImages] = useState<ImageAttachment[]>([])
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  const addFiles = useCallback(async (files: FileList | File[]) => {
    const fileArr = Array.from(files).filter(f => ACCEPTED_TYPES.includes(f.type))
    const attachments = await Promise.all(fileArr.map(fileToAttachment).map(p => p.catch(() => null)))
    const valid = attachments.filter((a): a is ImageAttachment => a !== null)
    if (valid.length) setImages(prev => [...prev, ...valid])
  }, [])

  const handleSend = () => {
    const trimmed = text.trim()
    if ((!trimmed && images.length === 0) || disabled) return
    onSend(trimmed, images.length > 0 ? images : undefined)
    setText('')
    setImages([])
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto'
    }
  }

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const handleInput = () => {
    const el = textareaRef.current
    if (el) {
      el.style.height = 'auto'
      el.style.height = `${Math.min(el.scrollHeight, 200)}px`
    }
  }

  const handlePaste = (e: ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items
    if (!items) return
    const imageFiles: File[] = []
    for (const item of Array.from(items)) {
      if (item.type.startsWith('image/')) {
        const file = item.getAsFile()
        if (file) imageFiles.push(file)
      }
    }
    if (imageFiles.length > 0) {
      e.preventDefault()
      addFiles(imageFiles)
    }
  }

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    if (e.dataTransfer?.files) {
      addFiles(e.dataTransfer.files)
    }
  }

  const handleDragOver = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
  }

  const removeImage = (index: number) => {
    setImages(prev => prev.filter((_, i) => i !== index))
  }

  return (
    <div className="border-t border-zinc-700 bg-zinc-900 px-4 py-3">
      {/* Image previews */}
      {images.length > 0 && (
        <div className="flex gap-2 mb-2 flex-wrap">
          {images.map((img, i) => (
            <div key={i} className="relative group">
              <img
                src={`data:${img.mediaType};base64,${img.data}`}
                alt="attachment"
                className="h-16 w-16 rounded-lg object-cover border border-zinc-600"
              />
              <button
                onClick={() => removeImage(i)}
                className="absolute -top-1.5 -right-1.5 w-5 h-5 rounded-full bg-zinc-700 text-zinc-300 text-xs flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-red-600"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}
      <div
        className="flex items-end gap-2 rounded-xl border border-zinc-600 bg-zinc-800 px-3 py-2 focus-within:border-[#39FF14]"
        onDrop={handleDrop}
        onDragOver={handleDragOver}
      >
        {/* Attach button */}
        <button
          onClick={() => fileInputRef.current?.click()}
          disabled={disabled}
          className="flex-shrink-0 text-zinc-400 hover:text-zinc-200 disabled:opacity-40 pb-0.5"
          title="Attach image"
        >
          📎
        </button>
        <input
          ref={fileInputRef}
          type="file"
          accept="image/png,image/jpeg,image/gif,image/webp"
          multiple
          className="hidden"
          onChange={e => {
            if (e.target.files) addFiles(e.target.files)
            e.target.value = ''
          }}
        />
        <textarea
          ref={textareaRef}
          value={text}
          onChange={e => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          onInput={handleInput}
          onPaste={handlePaste}
          placeholder={disabled ? 'Thinking\u2026' : 'Message Batchismo\u2026 (Enter to send, Shift+Enter for newline)'}
          disabled={disabled}
          rows={1}
          className="flex-1 resize-none bg-transparent text-sm text-zinc-100 placeholder-zinc-500 outline-none disabled:opacity-50"
          style={{ minHeight: '1.5rem', maxHeight: '200px' }}
        />
        <button
          onClick={handleSend}
          disabled={disabled || (!text.trim() && images.length === 0)}
          className="flex-shrink-0 rounded-lg bg-[#39FF14] px-3 py-1.5 text-sm font-medium text-black transition-colors hover:bg-[#2bcc10] disabled:opacity-40 disabled:cursor-not-allowed"
        >
          Send
        </button>
      </div>
    </div>
  )
}
