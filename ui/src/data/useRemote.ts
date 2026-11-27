import { useCallback, useEffect, useState } from 'react'

export type Remote<T> = { status: 'loading' } | { status: 'error'; error: unknown } | { status: 'ready'; data: T }

const isAbort = (error: unknown) => error instanceof DOMException && error.name === 'AbortError'

/**
 * Loads one resource on mount. `load` must be a stable function (module level). `reload` refreshes in
 * the background and keeps what is on screen if the refresh fails; `replace` writes data in directly
 * (for example from an SSE payload).
 */
export function useRemote<T>(load: (signal: AbortSignal) => Promise<T>): { state: Remote<T>; reload: () => void; replace: (data: T) => void } {
  const [state, setState] = useState<Remote<T>>({ status: 'loading' })
  const [tick, setTick] = useState(0)

  useEffect(() => {
    const controller = new AbortController()
    load(controller.signal).then(
      (data) => setState({ status: 'ready', data }),
      (error: unknown) => {
        if (!isAbort(error)) setState((prev) => (prev.status === 'ready' ? prev : { status: 'error', error }))
      },
    )
    return () => controller.abort()
  }, [load, tick])

  const reload = useCallback(() => setTick((n) => n + 1), [])
  const replace = useCallback((data: T) => setState({ status: 'ready', data }), [])
  return { state, reload, replace }
}
