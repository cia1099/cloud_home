import { useCallback, useEffect, useState } from 'react'
import { listFolder, searchFiles, type FileItem, type SearchItem } from '../api/files'

export type Listing<T> = { status: 'loading' } | { status: 'error'; error: unknown } | { status: 'ready'; items: T[] }

const isAbort = (error: unknown) => error instanceof DOMException && error.name === 'AbortError'

interface Loaded<T> {
  key: string
  items?: T[]
  error?: unknown
}

function toListing<T>(state: Loaded<T>, key: string): Listing<T> {
  if (state.key !== key) return { status: 'loading' }
  if (state.items) return { status: 'ready', items: state.items }
  return { status: 'error', error: state.error }
}

/**
 * Contents of one folder. Changing folder shows the loading state; `reload` refreshes in the
 * background and keeps what is on screen if the refresh fails.
 */
export function useFolderListing(folderId: string | null): { listing: Listing<FileItem>; reload: () => void } {
  const key = folderId ?? 'root'
  const [state, setState] = useState<Loaded<FileItem>>({ key: '' })
  const [tick, setTick] = useState(0)

  useEffect(() => {
    const controller = new AbortController()
    listFolder(folderId, controller.signal).then(
      (items) => setState({ key, items }),
      (error: unknown) => {
        if (!isAbort(error)) setState((prev) => (prev.key === key && prev.items ? prev : { key, error }))
      },
    )
    return () => controller.abort()
  }, [folderId, key, tick])

  const reload = useCallback(() => setTick((n) => n + 1), [])
  return { listing: toListing(state, key), reload }
}

interface SearchState extends Loaded<SearchItem> {
  page: number
  total: number
}

/** Search results (flat, whole drive), loaded a page at a time. A null query keeps the hook idle. */
export function useSearchListing(query: string | null) {
  const key = query ?? ''
  const [state, setState] = useState<SearchState>({ key: '\0', page: 1, total: 0 })
  const [tick, setTick] = useState(0)
  const [loadingMore, setLoadingMore] = useState(false)

  useEffect(() => {
    if (query === null) return
    const controller = new AbortController()
    searchFiles(query, 1, controller.signal).then(
      (result) => setState({ key, items: result.items, page: 1, total: result.total }),
      (error: unknown) => {
        if (!isAbort(error)) setState((prev) => (prev.key === key && prev.items ? prev : { key, error, page: 1, total: 0 }))
      },
    )
    return () => controller.abort()
  }, [query, key, tick])

  const reload = useCallback(() => setTick((n) => n + 1), [])

  const loadMore = useCallback(() => {
    if (query === null || !state.items || loadingMore) return
    setLoadingMore(true)
    searchFiles(query, state.page + 1)
      .then((result) =>
        setState((prev) =>
          prev.key === key ? { key, items: [...(prev.items ?? []), ...result.items], page: prev.page + 1, total: result.total } : prev,
        ),
      )
      .catch(() => undefined)
      .finally(() => setLoadingMore(false))
  }, [query, key, state, loadingMore])

  const listing = toListing<SearchItem>(state, key)
  const hasMore = listing.status === 'ready' && listing.items.length < state.total
  return { listing, total: state.total, hasMore, loadMore, loadingMore, reload }
}
