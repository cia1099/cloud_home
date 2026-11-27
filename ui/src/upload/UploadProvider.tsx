import { useCallback, useEffect, useMemo, useReducer, useRef, type ReactNode } from 'react'
import { errorKey } from '../api/errors'
import { ApiError } from '../api/client'
import { createFolder } from '../api/files'
import { uploadFile } from '../api/upload'
import { useOnline } from '../shell/useOnline'
import { INITIAL_STATE, dirKey, isActive, nextToStart, uploadReducer, type Transfer } from './model'
import { UploadContext, type UploadApi } from './useUpload'

const PROGRESS_INTERVAL_MS = 100
let sequence = 0
const nextId = (prefix: string) => `${prefix}${++sequence}`

/** Owns the upload queue for the whole signed-in app, so uploads keep going while the user browses. */
export function UploadProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(uploadReducer, INITIAL_STATE)
  const online = useOnline()
  const running = useRef(new Map<string, AbortController>())
  // Folders already created for a dropped tree: "group|path" → the promise of its id.
  const folders = useRef(new Map<string, Promise<string | null>>())

  const ensureFolders = useCallback((t: Transfer): Promise<string | null> => {
    const walk = async (): Promise<string | null> => {
      let parent = t.parentId
      for (let depth = 1; depth <= t.dirs.length; depth++) {
        const key = `${t.group}|${dirKey(t.dirs.slice(0, depth))}`
        const above = parent
        let made = folders.current.get(key)
        if (!made) {
          made = createFolder(t.dirs[depth - 1], above).then((folder) => {
            if (depth === 1 && t.group) dispatch({ type: 'group-named', group: t.group, name: folder.name })
            return folder.id
          })
          folders.current.set(key, made)
          made.catch(() => folders.current.delete(key)) // a retry creates it again
        }
        parent = await made
      }
      return parent
    }
    return walk()
  }, [])

  const run = useCallback(
    async (t: Transfer) => {
      const controller = new AbortController()
      running.current.set(t.id, controller)
      dispatch({ type: 'start', id: t.id })
      let last = 0
      try {
        const parentId = await ensureFolders(t)
        const saved = await uploadFile(
          t.file,
          parentId,
          (loaded) => {
            const now = performance.now()
            if (now - last < PROGRESS_INTERVAL_MS) return
            last = now
            dispatch({ type: 'progress', id: t.id, loaded })
          },
          controller.signal,
        )
        dispatch({ type: 'done', id: t.id, savedAs: saved.name })
      } catch (cause) {
        if (controller.signal.aborted) return // cancelled: the cancel action already updated the row
        if (cause instanceof ApiError && cause.code === 'NETWORK' && !navigator.onLine) dispatch({ type: 'pause', id: t.id })
        else dispatch({ type: 'fail', id: t.id, error: errorKey(cause) })
      } finally {
        running.current.delete(t.id)
      }
    },
    [ensureFolders],
  )

  // The queue: whenever something changes (new files, a finished transfer, back online) fill the free slots.
  useEffect(() => {
    if (!online) return
    for (const id of nextToStart(state.transfers)) {
      const t = state.transfers.find((item) => item.id === id)
      if (t && !running.current.has(id)) void run(t)
    }
  }, [state.transfers, online, run])

  const active = isActive(state.transfers)
  useEffect(() => {
    if (!active) return
    // Closing the tab would drop the transfers that are still running.
    const warn = (event: BeforeUnloadEvent) => event.preventDefault()
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [active])

  useEffect(() => {
    const controllers = running.current
    return () => controllers.forEach((controller) => controller.abort())
  }, [])

  const api = useMemo<UploadApi>(
    () => ({
      state,
      enqueue: (picked, destination) => {
        const groups = new Map<string, string>()
        const transfers = picked.map(({ file, dirs }): Transfer => {
          let group: string | null = null
          if (dirs.length > 0) {
            group = groups.get(dirs[0]) ?? nextId('g')
            groups.set(dirs[0], group)
          }
          return { id: nextId('u'), group, file, name: file.name, dirs, size: file.size, parentId: destination.id, destName: destination.name, status: 'waiting', loaded: 0, error: null, savedAs: null }
        })
        if (transfers.length > 0) dispatch({ type: 'add', transfers, now: Date.now() })
      },
      cancel: (ids) => {
        for (const id of ids) running.current.get(id)?.abort()
        dispatch({ type: 'cancel', ids: [...ids] })
      },
      retry: (ids) => dispatch({ type: 'retry', ids: [...ids], now: Date.now() }),
      clear: () => dispatch({ type: 'clear' }),
    }),
    [state],
  )

  return <UploadContext value={api}>{children}</UploadContext>
}
