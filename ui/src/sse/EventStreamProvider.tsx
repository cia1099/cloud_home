import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { fetchMe } from '../api/auth'
import { API_BASE } from '../api/client'
import { backoffDelay } from './backoff'
import { STREAM_EVENTS, StreamContext, type Listener, type StreamEvent, type StreamStatus } from './useEventStream'

/** One SSE connection for the signed-in app. The cookie authenticates it; no token is handled in JS. */
export function EventStreamProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<StreamStatus>('connecting')
  const listeners = useRef(new Map<StreamEvent, Set<Listener>>())

  useEffect(() => {
    let source: EventSource | undefined
    let timer: number | undefined
    let attempt = 0

    const connect = () => {
      const stream = new EventSource(`${API_BASE}/events/stream`)
      source = stream
      stream.onopen = () => {
        attempt = 0
        setStatus('open')
      }
      for (const name of STREAM_EVENTS) {
        stream.addEventListener(name, (event) => {
          let data: unknown
          try {
            data = JSON.parse((event as MessageEvent<string>).data)
          } catch {
            return
          }
          listeners.current.get(name)?.forEach((listener) => listener(data))
        })
      }
      stream.onerror = () => {
        setStatus('reconnecting')
        // The browser retries by itself while the connection is merely lost. A CLOSED stream
        // was refused (for example 401 after the session ended): probe /auth/me, which hands
        // a 401 to the session, then try again with backoff.
        if (stream.readyState !== EventSource.CLOSED) return
        stream.close()
        void fetchMe().catch(() => undefined)
        timer = window.setTimeout(connect, backoffDelay(attempt++))
      }
    }

    connect()
    return () => {
      window.clearTimeout(timer)
      source?.close()
    }
  }, [])

  const subscribe = useCallback((event: StreamEvent, listener: Listener) => {
    const set = listeners.current.get(event) ?? new Set<Listener>()
    set.add(listener)
    listeners.current.set(event, set)
    return () => {
      set.delete(listener)
    }
  }, [])

  const value = useMemo(() => ({ status, subscribe }), [status, subscribe])
  return <StreamContext value={value}>{children}</StreamContext>
}
