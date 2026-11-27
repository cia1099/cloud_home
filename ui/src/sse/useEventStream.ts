import { createContext, useContext, useEffect, useRef } from 'react'

/** Event names sent by `GET /events/stream`. */
export type StreamEvent = 'usage' | 'files' | 'update'
export const STREAM_EVENTS: readonly StreamEvent[] = ['usage', 'files', 'update']

export type StreamStatus = 'connecting' | 'open' | 'reconnecting'

export type Listener = (data: unknown) => void

export interface Stream {
  status: StreamStatus
  subscribe: (event: StreamEvent, listener: Listener) => () => void
}

export const StreamContext = createContext<Stream | null>(null)

function useStream(): Stream {
  const value = useContext(StreamContext)
  if (!value) throw new Error('useStream outside EventStreamProvider')
  return value
}

export function useStreamStatus(): StreamStatus {
  return useStream().status
}

/** Calls `listener` with the parsed JSON payload of every `event` while the component is mounted. */
export function useStreamEvent(event: StreamEvent, listener: Listener): void {
  const { subscribe } = useStream()
  const latest = useRef(listener)
  useEffect(() => {
    latest.current = listener
  })
  useEffect(() => subscribe(event, (data) => latest.current(data)), [subscribe, event])
}
