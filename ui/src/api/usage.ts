import { api } from './client'

/** `GET /usage` and the payload of the SSE `usage` event. */
export interface Usage {
  used_bytes: number
  trashed_bytes: number
  file_count: number
  volume_total_bytes: number
  volume_available_bytes: number
  drive_available: boolean
}

export const getUsage = (signal?: AbortSignal) => api<Usage>('/usage', { signal })
