import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import type { BatEvent } from '../types'

export function useBatEvents(onEvent: (event: BatEvent) => void) {
  useEffect(() => {
    const unlisten = listen<BatEvent>('bat-event', (event) => {
      onEvent(event.payload)
    })
    return () => { unlisten.then(fn => fn()) }
  }, [onEvent])
}
