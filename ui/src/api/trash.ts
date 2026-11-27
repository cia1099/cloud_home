import { api } from './client'
import type { FileItem } from './files'

/** One entry of `GET /trash`; `id` is the trash entry (restore and delete use it), `file_id` the file. */
export interface TrashItem {
  id: string
  file_id: string
  name: string
  file_type: 'file' | 'folder'
  size_bytes: number
  deleted_at: string
  expires_at: string
}

export const listTrash = (signal?: AbortSignal) => api<{ items: TrashItem[] }>('/trash', { signal }).then((r) => r.items)

/** Returns the restored file; its `parent_id` tells where it landed (null = the root). */
export const restoreTrashItem = (id: string) => api<FileItem>(`/trash/${encodeURIComponent(id)}/restore`, { method: 'POST' })

export const deleteForever = (id: string) => api<{ freed_bytes: number }>(`/trash/${encodeURIComponent(id)}`, { method: 'DELETE' })

export const emptyTrash = () => api<{ deleted_count: number; freed_bytes: number }>('/trash', { method: 'DELETE' })
