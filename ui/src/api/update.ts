import { API_BASE, ApiError, api } from './client'

export type Phase = 'idle' | 'checking' | 'downloading' | 'installing' | 'restarting' | 'failed'
export type Mode = 'manual' | 'auto'
export type Channel = 'stable' | 'beta'

export interface UpdateSettings {
  mode: Mode
  /** `HH:MM-HH:MM`, local time of the box. */
  window: string
  channel: Channel
  check_interval_hours: number
}

export interface UpdateStatus {
  /** False when the box does not run from an install folder (a development copy): nothing can be installed. */
  enabled: boolean
  phase: Phase
  /** The one product version: server and embedded web app. */
  current_version: string
  available: { version: string; notes_url: string; size: number } | null
  last_checked_at: string | null
  last_error: string | null
  settings: UpdateSettings
}

/** The administration endpoints take the shared admin password (`X-Admin-Password`), never the session cookie alone. */
export const fetchUpdateStatus = (password: string, signal?: AbortSignal) => api<UpdateStatus>('/admin/update', { adminPassword: password, signal })

export const checkForUpdate = (password: string) => api<UpdateStatus>('/admin/update/check', { method: 'POST', adminPassword: password })

export const installUpdate = (password: string, force: boolean) =>
  api<UpdateStatus>('/admin/update/install', { method: 'POST', adminPassword: password, body: { force } })

export const saveUpdateSettings = (password: string, change: Partial<Pick<UpdateSettings, 'mode' | 'window' | 'channel'>>) =>
  api<UpdateStatus>('/admin/update/settings', { method: 'PUT', adminPassword: password, body: change })

/**
 * Whether this caller can manage the box, without a password: 401 means the endpoint is there and wants
 * the password (a local caller on a box with an admin password); 404 and 403 mean it is not for this caller.
 */
export async function probeAdmin(signal?: AbortSignal): Promise<boolean> {
  try {
    await fetchUpdateStatus('', signal)
    return true
  } catch (error) {
    return error instanceof ApiError && error.status === 401
  }
}

/** The version the box is running now (`GET /health`), or null when it does not answer. */
export async function fetchServerVersion(signal?: AbortSignal): Promise<string | null> {
  try {
    const response = await fetch(`${API_BASE}/health`, { credentials: 'same-origin', cache: 'no-store', signal })
    if (!response.ok) return null
    const payload = (await response.json()) as { version?: unknown }
    return typeof payload.version === 'string' ? payload.version : null
  } catch {
    return null
  }
}
