import ReactMarkdown from 'react-markdown'

interface Props {
  text: string
}

export function StreamingText({ text }: Props) {
  if (!text) return null
  return (
    <div className="rounded-2xl bg-zinc-800 px-4 py-3 text-zinc-100 max-w-[85%]">
      <div className="prose prose-invert prose-sm max-w-none">
        <ReactMarkdown>{text}</ReactMarkdown>
      </div>
      <span className="inline-block w-1 h-4 bg-zinc-400 animate-pulse align-middle ml-0.5" />
    </div>
  )
}
