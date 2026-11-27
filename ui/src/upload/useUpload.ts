import { createContext, useContext } from 'react'
import type { Picked, UploadState } from './model'

export interface UploadApi {
  state: UploadState
  /** Queues files for a folder (`id` null = the root). */
  enqueue: (picked: Picked[], destination: { id: string | null; name: string }) => void
  cancel: (ids: readonly string[]) => void
  retry: (ids: readonly string[]) => void
  /** Removes finished, failed and cancelled rows from the panel. */
  clear: () => void
}

export const UploadContext = createContext<UploadApi | null>(null)

export function useUpload(): UploadApi {
  const value = useContext(UploadContext)
  if (!value) throw new Error('useUpload outside UploadProvider')
  return value
}
