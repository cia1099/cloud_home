import { describe, expect, it } from 'vitest'
import { galleryOf, mediaKind, neighbor, stripWindow, swipeDelta } from './model'

const file = (id: string, name: string, mime: string | null) => ({ id, name, file_type: 'file' as const, mime_type: mime })
const items = [
  file('a', 'a.jpg', 'image/jpeg'),
  file('doc', 'n.txt', 'text/plain'),
  file('b', 'b.png', 'image/png'),
  file('song', 's.mp3', 'audio/mpeg'),
  file('c', 'c.webp', 'image/webp'),
  { id: 'dir', name: 'dir', file_type: 'folder' as const, mime_type: null },
]

describe('preview model', () => {
  it('classifies previewable files', () => {
    expect(items.map(mediaKind)).toEqual(['image', null, 'image', 'audio', 'image', null])
    expect(mediaKind(file('v', 'v.mp4', 'video/mp4'))).toBe('video')
  })

  it('galleries only the images, in display order; media plays alone', () => {
    expect(galleryOf(items, items[2]).map((i) => i.id)).toEqual(['a', 'b', 'c'])
    expect(galleryOf(items, items[3]).map((i) => i.id)).toEqual(['song'])
  })

  it('steps through ids and wraps around', () => {
    const ids = ['a', 'b', 'c']
    expect(neighbor(ids, 'a', 1)).toBe('b')
    expect(neighbor(ids, 'c', 1)).toBe('a')
    expect(neighbor(ids, 'a', -1)).toBe('c')
    expect(neighbor(['a'], 'a', 1)).toBeNull()
    expect(neighbor(ids, 'zzz', 1)).toBeNull()
  })

  it('reads swipes', () => {
    expect(swipeDelta(-80, 5)).toBe(1)
    expect(swipeDelta(80, -5)).toBe(-1)
    expect(swipeDelta(30, 0)).toBeNull()
    expect(swipeDelta(60, 120)).toBeNull()
  })

  it('windows the thumbnail strip', () => {
    expect(stripWindow(1000, 500, 10)).toEqual({ from: 490, to: 511 })
    expect(stripWindow(1000, 0, 10)).toEqual({ from: 0, to: 21 })
    expect(stripWindow(1000, 999, 10)).toEqual({ from: 979, to: 1000 })
    expect(stripWindow(5, 2, 10)).toEqual({ from: 0, to: 5 })
  })
})
