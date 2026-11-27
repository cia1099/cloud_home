import type { Usage } from '../api/usage'

export type SegmentId = 'mine' | 'trash' | 'others' | 'free'

export interface Segment {
  id: SegmentId
  bytes: number
}

/** Splits the whole drive into your files, your trash, other accounts' data and free space. */
export function segments(usage: Usage): Segment[] {
  const total = usage.volume_total_bytes
  const free = Math.min(usage.volume_available_bytes, total)
  const others = Math.max(0, total - free - usage.used_bytes - usage.trashed_bytes)
  return [
    { id: 'mine', bytes: usage.used_bytes },
    { id: 'trash', bytes: usage.trashed_bytes },
    { id: 'others', bytes: others },
    { id: 'free', bytes: free },
  ]
}

/** Width of a segment in percent of the drive (0 when the drive reports no capacity). */
export const percentOf = (bytes: number, total: number) => (total > 0 ? Math.min(100, (bytes / total) * 100) : 0)

/** Under 10 % free shows the amber warning; a missing drive is a different state. */
export const isLow = (usage: Usage) => usage.drive_available && usage.volume_total_bytes > 0 && usage.volume_available_bytes / usage.volume_total_bytes < 0.1
