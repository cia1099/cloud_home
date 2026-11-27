import { api } from './client'

/** A file or folder as returned by `GET /files`, `GET /files/:id` and the mutating endpoints. */
export interface FileItem {
  id: string
  name: string
  file_type: 'file' | 'folder'
  size_bytes: number
  mime_type: string | null
  parent_id: string | null
  created_at: string
  updated_at: string
}

/** A search hit: the item plus the path of its parent folder (empty for the root). */
export interface SearchItem extends FileItem {
  parent_path: string
}

export interface SearchPage {
  items: SearchItem[]
  total: number
}

export const SEARCH_PAGE_SIZE = 100

export const listFolder = (parentId: string | null, signal?: AbortSignal) =>
  api<{ items: FileItem[] }>(`/files${parentId ? `?parent_id=${encodeURIComponent(parentId)}` : ''}`, { signal }).then((r) => r.items)

export const getFile = (id: string, signal?: AbortSignal) => api<FileItem>(`/files/${encodeURIComponent(id)}`, { signal })

export const renameFile = (id: string, name: string) =>
  api<FileItem>(`/files/${encodeURIComponent(id)}`, { method: 'PATCH', body: { name } })

export const moveFile = (id: string, targetParentId: string | null) =>
  api<FileItem>(`/files/${encodeURIComponent(id)}/move`, { method: 'POST', body: { target_parent_id: targetParentId } })

export const trashFile = (id: string) => api<void>(`/files/${encodeURIComponent(id)}`, { method: 'DELETE' })

export const createFolder = (name: string, parentId: string | null) =>
  api<FileItem>('/folders', { method: 'POST', body: { name, parent_id: parentId } })

/** Undo for "Move to Trash": the restore endpoint takes the trash entry id, so look it up by file id first. */
export async function restoreFromTrash(fileId: string): Promise<void> {
  const { items } = await api<{ items: { id: string; file_id: string }[] }>('/trash')
  const entry = items.find((item) => item.file_id === fileId)
  if (entry) await api<unknown>(`/trash/${encodeURIComponent(entry.id)}/restore`, { method: 'POST' })
}

export async function searchFiles(query: string, page: number, signal?: AbortSignal): Promise<SearchPage> {
  const params = new URLSearchParams({ q: query, page: String(page), per_page: String(SEARCH_PAGE_SIZE) })
  const result = await api<{ items: SearchItem[]; pagination: { total: number } }>(`/search?${params}`, { signal })
  return { items: result.items, total: result.pagination.total }
}

/** JPEG thumbnail; the server only makes them for images. */
export const thumbnailUrl = (id: string) => `/api/v1/files/${encodeURIComponent(id)}/thumbnail?size=256`
