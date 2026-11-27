import { api } from './client'

export interface Person {
  id: string
  display_name: string
}

/** A share the signed-in user created. Targeted shares (`recipients` not empty) have no token. */
export interface Share {
  id: string
  file_id: string
  token: string | null
  recipients: Person[]
  can_download: boolean
  expires_at: string | null
  access_count: number
  created_at: string
  is_active: boolean
}

/** A share addressed to the signed-in user. */
export interface ReceivedShare {
  id: string
  file_id: string
  name: string
  file_type: 'file' | 'folder'
  mime_type: string | null
  size_bytes: number
  child_count: number | null
  owner: Person
  created_at: string
  expires_at: string | null
  can_download: boolean
  seen_at: string | null
}

export interface ShareRequest {
  file_id: string
  can_download: boolean
  expires_at: string | null
  recipient_ids: string[]
}

export interface PublicShare {
  name: string
  file_type: 'file' | 'folder'
  size_bytes: number
  mime_type: string | null
  can_download: boolean
  shared_by: string
}

const enc = encodeURIComponent

export const createShare = (request: ShareRequest) => api<Share>('/shares', { method: 'POST', body: request })

/** Live shares only: the server keeps revoked rows, the UI does not show them. */
export const listShares = (signal?: AbortSignal) =>
  api<{ items: Share[] }>('/shares', { signal }).then((r) => r.items.filter((share) => share.is_active))

export const revokeShare = (id: string) => api<void>(`/shares/${enc(id)}`, { method: 'DELETE' })

export const listReceived = (signal?: AbortSignal) => api<{ items: ReceivedShare[] }>('/shares/received', { signal }).then((r) => r.items)

export const markSeen = (id: string) => api<void>(`/shares/${enc(id)}/seen`, { method: 'POST' })

export const listDirectory = (signal?: AbortSignal) => api<{ items: Person[] }>('/users/directory', { signal }).then((r) => r.items)

export const getPublicShare = (token: string, signal?: AbortSignal) => api<PublicShare>(`/public/shares/${enc(token)}`, { signal })

export const publicDownloadUrl = (token: string) => `/api/v1/public/shares/${enc(token)}/download`

export const receivedDownloadUrl = (shareId: string, fileId: string) => `/api/v1/shares/received/${enc(shareId)}/files/${enc(fileId)}/download`
