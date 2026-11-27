import type { FileItem } from '../api/files'

export type SortKey = 'name' | 'modified' | 'size'
export type SortDir = 'asc' | 'desc'
export interface Sort {
  key: SortKey
  dir: SortDir
}
export const DEFAULT_SORT: Sort = { key: 'name', dir: 'asc' }

export type Kind = 'folder' | 'image' | 'video' | 'audio' | 'pdf' | 'doc' | 'sheet' | 'zip' | 'file'

const isFolder = (item: Pick<FileItem, 'file_type'>) => item.file_type === 'folder'

/** Name sort is natural ("file2" before "file10") and locale-aware; folders always come first. */
export function sortItems<T extends FileItem>(items: readonly T[], sort: Sort, intlTag: string): T[] {
  const collator = new Intl.Collator(intlTag, { numeric: true, sensitivity: 'base' })
  const sign = sort.dir === 'asc' ? 1 : -1
  const byName = (a: T, b: T) => collator.compare(a.name, b.name)
  return [...items].sort((a, b) => {
    if (isFolder(a) !== isFolder(b)) return isFolder(a) ? -1 : 1
    let order = 0
    if (sort.key === 'name') order = byName(a, b)
    else if (sort.key === 'size') order = isFolder(a) ? 0 : a.size_bytes - b.size_bytes
    else order = Date.parse(a.updated_at) - Date.parse(b.updated_at)
    return order !== 0 ? order * sign : byName(a, b)
  })
}

/** Splits at the last dot; a leading dot (".profile") is part of the base name. Folders have no extension. */
export function splitName(item: Pick<FileItem, 'name' | 'file_type'>): { base: string; ext: string } {
  const dot = item.name.lastIndexOf('.')
  if (item.file_type === 'folder' || dot <= 0 || dot === item.name.length - 1) return { base: item.name, ext: '' }
  return { base: item.name.slice(0, dot), ext: item.name.slice(dot) }
}

const SHEET_EXT = new Set(['.xls', '.xlsx', '.csv', '.numbers', '.ods'])
const DOC_EXT = new Set(['.doc', '.docx', '.txt', '.md', '.rtf', '.pages', '.odt', '.ppt', '.pptx', '.key'])
const ZIP_EXT = new Set(['.zip', '.gz', '.tar', '.tgz', '.7z', '.rar', '.bz2', '.xz'])

/** Picks the icon family from the MIME type, falling back to the extension. */
export function kindOf(item: Pick<FileItem, 'file_type' | 'name' | 'mime_type'>): Kind {
  if (isFolder(item)) return 'folder'
  const mime = item.mime_type ?? ''
  if (mime.startsWith('image/')) return 'image'
  if (mime.startsWith('video/')) return 'video'
  if (mime.startsWith('audio/')) return 'audio'
  if (mime === 'application/pdf') return 'pdf'
  const ext = splitName(item).ext.toLowerCase()
  if (SHEET_EXT.has(ext)) return 'sheet'
  if (ZIP_EXT.has(ext) || mime === 'application/zip') return 'zip'
  if (DOC_EXT.has(ext) || mime.startsWith('text/')) return 'doc'
  return 'file'
}

/** Ids between `anchor` and `target` (inclusive) in display order, for Shift-click. */
export function rangeIds(order: readonly string[], anchor: string, target: string): string[] {
  const a = order.indexOf(anchor)
  const b = order.indexOf(target)
  if (a < 0 || b < 0) return [target]
  return order.slice(Math.min(a, b), Math.max(a, b) + 1)
}

export function toggleId(selection: ReadonlySet<string>, id: string): Set<string> {
  const next = new Set(selection)
  if (!next.delete(id)) next.add(id)
  return next
}

/** The visible slice of a fixed-height row list, with `overscan` extra rows on both sides. */
export function visibleRange(scrollTop: number, viewport: number, rowHeight: number, rowCount: number, overscan = 4) {
  const first = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan)
  const last = Math.min(rowCount, Math.ceil((scrollTop + viewport) / rowHeight) + overscan)
  return { first, last }
}

/** Grid tiles per row for a container width (CSS: minmax(168px, 1fr) with 16 px gaps and 24 px padding). */
export function gridColumns(width: number, minTile = 168, gap = 16, padding = 24): number {
  return Math.max(1, Math.floor((width - padding * 2 + gap) / (minTile + gap)))
}

export function chunk<T>(items: readonly T[], size: number): T[][] {
  const rows: T[][] = []
  for (let i = 0; i < items.length; i += size) rows.push(items.slice(i, i + size))
  return rows
}

export interface Crumb {
  id: string
  name: string
}

/** Walks `parent_id` upwards from `folder`; stops at the root, on a cycle or after `limit` levels. */
export async function ancestorsOf(
  folder: Pick<FileItem, 'id' | 'name' | 'parent_id'>,
  fetchParent: (id: string) => Promise<Pick<FileItem, 'id' | 'name' | 'parent_id'>>,
  limit = 32,
): Promise<Crumb[]> {
  const path: Crumb[] = [{ id: folder.id, name: folder.name }]
  const seen = new Set([folder.id])
  let parentId = folder.parent_id
  while (parentId && path.length < limit && !seen.has(parentId)) {
    const parent = await fetchParent(parentId)
    seen.add(parent.id)
    path.unshift({ id: parent.id, name: parent.name })
    parentId = parent.parent_id
  }
  return path
}

/**
 * Does an SSE `files` event (payload `{ kind, file, parent_ids }`) invalidate the listing of `folderId`
 * (null = root)? An empty or malformed scope means "unknown", so refresh.
 */
export function affectsFolder(event: unknown, folderId: string | null): boolean {
  const ids = typeof event === 'object' && event !== null ? (event as { parent_ids?: unknown }).parent_ids : undefined
  return !Array.isArray(ids) || ids.length === 0 || ids.includes(folderId)
}
