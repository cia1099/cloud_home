import type { FileItem } from '../api/files'
import { kindOf } from '../files/model'

export type MediaKind = 'image' | 'audio' | 'video'

/** The preview kind of a file, or null when it has no in-browser preview (it is downloaded instead). */
export function mediaKind(item: Pick<FileItem, 'file_type' | 'name' | 'mime_type'>): MediaKind | null {
  const kind = kindOf(item)
  return kind === 'image' || kind === 'audio' || kind === 'video' ? kind : null
}

/** What the gallery steps through: the images in display order for an image, otherwise just the file itself. */
export function galleryOf<T extends Pick<FileItem, 'id' | 'file_type' | 'name' | 'mime_type'>>(items: readonly T[], current: T): T[] {
  return mediaKind(current) === 'image' ? items.filter((item) => mediaKind(item) === 'image') : [current]
}

/** The id `delta` steps from `id`, wrapping around at both ends; null when there is nothing else to show. */
export function neighbor(ids: readonly string[], id: string, delta: number): string | null {
  const index = ids.indexOf(id)
  if (index < 0 || ids.length < 2) return null
  return ids[(((index + delta) % ids.length) + ids.length) % ids.length]
}

const SWIPE_MIN = 50

/** A horizontal swipe moves through the gallery (left = next); mostly vertical or short drags do nothing. */
export function swipeDelta(dx: number, dy: number): 1 | -1 | null {
  if (Math.abs(dx) < SWIPE_MIN || Math.abs(dx) < Math.abs(dy) * 1.5) return null
  return dx < 0 ? 1 : -1
}

/** The slice of thumbnails to render around the current one, so a folder of thousands of photos stays light. */
export function stripWindow(length: number, index: number, radius: number): { from: number; to: number } {
  const size = radius * 2 + 1
  const from = Math.max(0, Math.min(index - radius, length - size))
  return { from, to: Math.min(length, from + size) }
}
