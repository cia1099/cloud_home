import type { MessageKey } from '../i18n/messages/en'

/** A file picked or dropped, with the folders above it relative to the dropped top-level folder (empty for a loose file). */
export interface Picked {
  file: File
  dirs: string[]
}

export type Status = 'waiting' | 'uploading' | 'done' | 'failed' | 'cancelled'

export interface Transfer {
  id: string
  /** Id of the dropped top-level folder this file belongs to; null for loose files. */
  group: string | null
  /** Only a reference: the browser streams the bytes from disk while sending. */
  file: File
  name: string
  dirs: string[]
  size: number
  /** Destination folder id (null = the root) and its display name. */
  parentId: string | null
  destName: string
  status: Status
  loaded: number
  error: MessageKey | null
  /** The name the server stored the file under (it may differ from `name`). */
  savedAs: string | null
}

export interface UploadState {
  transfers: Transfer[]
  /** The names the server gave to dropped top-level folders, by group id. */
  groupNames: Record<string, string>
  /** When the current burst of work began (for the remaining-time estimate). */
  startedAt: number | null
}

export const INITIAL_STATE: UploadState = { transfers: [], groupNames: {}, startedAt: null }

/** Concurrent transfers: few enough that each large file gets the full link (design/app-spec.md section 4). */
export const MAX_CONCURRENT = 2

export type UploadAction =
  | { type: 'add'; transfers: Transfer[]; now: number }
  | { type: 'start'; id: string }
  | { type: 'progress'; id: string; loaded: number }
  | { type: 'done'; id: string; savedAs: string }
  | { type: 'fail'; id: string; error: MessageKey }
  | { type: 'pause'; id: string }
  | { type: 'cancel'; ids: string[] }
  | { type: 'retry'; ids: string[]; now: number }
  | { type: 'group-named'; group: string; name: string }
  | { type: 'clear' }

export const isActive = (transfers: readonly Transfer[]): boolean => transfers.some((t) => t.status === 'waiting' || t.status === 'uploading')

const patch = (state: UploadState, id: string, change: Partial<Transfer>): UploadState => ({
  ...state,
  transfers: state.transfers.map((t) => (t.id === id ? { ...t, ...change } : t)),
})

export function uploadReducer(state: UploadState, action: UploadAction): UploadState {
  switch (action.type) {
    case 'add':
      return { ...state, transfers: [...state.transfers, ...action.transfers], startedAt: isActive(state.transfers) ? state.startedAt : action.now }
    case 'start':
      return patch(state, action.id, { status: 'uploading', loaded: 0, error: null })
    case 'progress':
      return patch(state, action.id, { loaded: action.loaded })
    case 'done':
      return { ...state, transfers: state.transfers.map((t): Transfer => (t.id === action.id ? { ...t, status: 'done', loaded: t.size, savedAs: action.savedAs } : t)) }
    case 'fail':
      return patch(state, action.id, { status: 'failed', error: action.error })
    case 'pause':
      return patch(state, action.id, { status: 'waiting', loaded: 0 })
    case 'cancel': {
      const ids = new Set(action.ids)
      return { ...state, transfers: state.transfers.map((t) => (ids.has(t.id) && (t.status === 'waiting' || t.status === 'uploading') ? { ...t, status: 'cancelled', loaded: 0 } : t)) }
    }
    case 'retry': {
      const ids = new Set(action.ids)
      const transfers = state.transfers.map((t): Transfer => (ids.has(t.id) && (t.status === 'failed' || t.status === 'cancelled') ? { ...t, status: 'waiting', loaded: 0, error: null } : t))
      return { ...state, transfers, startedAt: isActive(state.transfers) ? state.startedAt : action.now }
    }
    case 'group-named':
      return { ...state, groupNames: { ...state.groupNames, [action.group]: action.name } }
    case 'clear':
      return { ...state, transfers: state.transfers.filter((t) => t.status === 'waiting' || t.status === 'uploading') }
  }
}

/** Ids of the waiting transfers to start now so that at most `limit` run at once (queue order). */
export function nextToStart(transfers: readonly Transfer[], limit: number = MAX_CONCURRENT): string[] {
  const running = transfers.filter((t) => t.status === 'uploading').length
  return transfers
    .filter((t) => t.status === 'waiting')
    .slice(0, Math.max(0, limit - running))
    .map((t) => t.id)
}

/** Key of a folder inside a dropped tree. Names cannot contain "/", so joining is unambiguous. */
export const dirKey = (dirs: readonly string[]): string => dirs.join('/')

export interface Row {
  id: string
  kind: 'file' | 'folder'
  name: string
  status: Status
  loaded: number
  size: number
  /** Files in a folder row. */
  count: number
  done: number
  /** The name the server stored it under, once known. */
  savedAs: string | null
  error: MessageKey | null
  transferIds: string[]
}

const rank = (status: Status): number => ['uploading', 'waiting', 'failed', 'cancelled', 'done'].indexOf(status)

/** One row per loose file and one per dropped folder (design/app-spec.md section 4). */
export function toRows(state: UploadState): Row[] {
  const rows: Row[] = []
  const byGroup = new Map<string, Row>()
  for (const t of state.transfers) {
    if (t.group === null) {
      rows.push({ id: t.id, kind: 'file', name: t.name, status: t.status, loaded: t.loaded, size: t.size, count: 1, done: t.status === 'done' ? 1 : 0, savedAs: t.savedAs, error: t.error, transferIds: [t.id] })
      continue
    }
    const row = byGroup.get(t.group)
    if (!row) {
      const created: Row = { id: t.group, kind: 'folder', name: t.dirs[0], status: t.status, loaded: t.loaded, size: t.size, count: 1, done: t.status === 'done' ? 1 : 0, savedAs: state.groupNames[t.group] ?? null, error: t.error, transferIds: [t.id] }
      byGroup.set(t.group, created)
      rows.push(created)
      continue
    }
    row.loaded += t.loaded
    row.size += t.size
    row.count += 1
    if (t.status === 'done') row.done += 1
    row.error = row.error ?? t.error
    if (rank(t.status) < rank(row.status)) row.status = t.status
    row.transferIds.push(t.id)
  }
  return rows
}

/** Bytes and counts over everything that still counts (cancelled files are ignored). */
export function overall(transfers: readonly Transfer[]): { loaded: number; total: number; remaining: number } {
  let loaded = 0
  let total = 0
  let remaining = 0
  for (const t of transfers) {
    if (t.status === 'cancelled') continue
    loaded += t.loaded
    total += t.size
    if (t.status === 'waiting' || t.status === 'uploading') remaining += 1
  }
  return { loaded, total, remaining }
}

/** Seconds left at the average speed so far; null until there is enough to estimate from. */
export function remainingSeconds(loaded: number, total: number, elapsedMs: number): number | null {
  if (loaded <= 0 || elapsedMs < 2000 || loaded >= total) return null
  return Math.ceil((total - loaded) / (loaded / (elapsedMs / 1000)))
}

/** Rounds a duration up to one readable unit (45 s, 3 min, 2 h) for `Intl.NumberFormat`. */
export function durationParts(seconds: number): { value: number; unit: 'second' | 'minute' | 'hour' } {
  if (seconds < 60) return { value: Math.max(1, Math.ceil(seconds)), unit: 'second' }
  if (seconds < 3600) return { value: Math.ceil(seconds / 60), unit: 'minute' }
  return { value: Math.ceil(seconds / 3600), unit: 'hour' }
}

/** True when the server stored the file under a different name than the local one (`a.txt` → `a(1).txt`). */
export const wasRenamed = (name: string, savedAs: string | null): boolean => savedAs !== null && savedAs !== name
