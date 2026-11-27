import { describe, expect, it } from 'vitest'
import type { FileItem } from '../api/files'
import { affectsFolder, ancestorsOf, chunk, gridColumns, kindOf, rangeIds, sortItems, splitName, toggleId, visibleRange } from './model'

function item(name: string, extra: Partial<FileItem> = {}): FileItem {
  return {
    id: name,
    name,
    file_type: 'file',
    size_bytes: 0,
    mime_type: null,
    parent_id: null,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...extra,
  }
}
const folder = (name: string, extra: Partial<FileItem> = {}) => item(name, { file_type: 'folder', ...extra })

describe('sortItems', () => {
  it('puts folders first and sorts names naturally', () => {
    const sorted = sortItems([item('file10.txt'), folder('Zeta'), item('file2.txt'), folder('Alpha')], { key: 'name', dir: 'asc' }, 'en')
    expect(sorted.map((i) => i.name)).toEqual(['Alpha', 'Zeta', 'file2.txt', 'file10.txt'])
  })

  it('keeps folders first when the direction is descending', () => {
    const sorted = sortItems([item('a'), folder('b'), item('c')], { key: 'name', dir: 'desc' }, 'en')
    expect(sorted.map((i) => i.name)).toEqual(['b', 'c', 'a'])
  })

  it('sorts by size and by modification time, breaking ties by name', () => {
    const files = [item('b', { size_bytes: 5 }), item('a', { size_bytes: 5 }), item('c', { size_bytes: 1 })]
    expect(sortItems(files, { key: 'size', dir: 'asc' }, 'en').map((i) => i.name)).toEqual(['c', 'a', 'b'])
    expect(sortItems(files, { key: 'size', dir: 'desc' }, 'en').map((i) => i.name)).toEqual(['a', 'b', 'c'])
    const dated = [item('old', { updated_at: '2020-01-01T00:00:00Z' }), item('new', { updated_at: '2026-01-01T00:00:00Z' })]
    expect(sortItems(dated, { key: 'modified', dir: 'desc' }, 'en').map((i) => i.name)).toEqual(['new', 'old'])
  })

  it('does not change the input array', () => {
    const input = [item('b'), item('a')]
    sortItems(input, { key: 'name', dir: 'asc' }, 'en')
    expect(input.map((i) => i.name)).toEqual(['b', 'a'])
  })

  it('collates Chinese names with the locale rules', () => {
    const sorted = sortItems([item('丙.txt'), item('甲.txt'), item('乙.txt')], { key: 'name', dir: 'asc' }, 'zh-Hans-CN')
    expect(sorted.map((i) => i.name)).toEqual(['丙.txt', '甲.txt', '乙.txt'].sort(new Intl.Collator('zh-Hans-CN').compare))
  })
})

describe('splitName', () => {
  it('separates the last extension', () => {
    expect(splitName(item('photo.jpg'))).toEqual({ base: 'photo', ext: '.jpg' })
    expect(splitName(item('archive.tar.gz'))).toEqual({ base: 'archive.tar', ext: '.gz' })
  })
  it('keeps dotfiles, trailing dots and folders whole', () => {
    expect(splitName(item('.profile'))).toEqual({ base: '.profile', ext: '' })
    expect(splitName(item('name.'))).toEqual({ base: 'name.', ext: '' })
    expect(splitName(item('README'))).toEqual({ base: 'README', ext: '' })
    expect(splitName(folder('v1.2'))).toEqual({ base: 'v1.2', ext: '' })
  })
})

describe('kindOf', () => {
  it('uses the MIME type first and the extension second', () => {
    expect(kindOf(folder('x'))).toBe('folder')
    expect(kindOf(item('a', { mime_type: 'image/png' }))).toBe('image')
    expect(kindOf(item('a', { mime_type: 'video/mp4' }))).toBe('video')
    expect(kindOf(item('a', { mime_type: 'audio/mpeg' }))).toBe('audio')
    expect(kindOf(item('a.pdf', { mime_type: 'application/pdf' }))).toBe('pdf')
    expect(kindOf(item('data.CSV', { mime_type: 'text/csv' }))).toBe('sheet')
    expect(kindOf(item('x.zip'))).toBe('zip')
    expect(kindOf(item('notes.txt', { mime_type: 'text/plain' }))).toBe('doc')
    expect(kindOf(item('blob.bin'))).toBe('file')
  })
})

describe('selection helpers', () => {
  const order = ['a', 'b', 'c', 'd', 'e']
  it('selects a range in either direction', () => {
    expect(rangeIds(order, 'b', 'd')).toEqual(['b', 'c', 'd'])
    expect(rangeIds(order, 'd', 'b')).toEqual(['b', 'c', 'd'])
    expect(rangeIds(order, 'zzz', 'c')).toEqual(['c'])
  })
  it('toggles without mutating', () => {
    const start = new Set(['a'])
    expect([...toggleId(start, 'b')]).toEqual(['a', 'b'])
    expect([...toggleId(start, 'a')]).toEqual([])
    expect([...start]).toEqual(['a'])
  })
})

describe('windowing', () => {
  it('renders the visible rows plus the overscan', () => {
    expect(visibleRange(0, 500, 50, 1000)).toEqual({ first: 0, last: 14 })
    expect(visibleRange(5000, 500, 50, 1000)).toEqual({ first: 96, last: 114 })
    expect(visibleRange(49_900, 500, 50, 1000)).toEqual({ first: 994, last: 1000 })
  })
  it('fits as many grid tiles as the width allows, at least one', () => {
    expect(gridColumns(1000)).toBe(5)
    expect(gridColumns(300)).toBe(1)
    expect(gridColumns(0)).toBe(1)
  })
  it('chunks items into rows', () => {
    expect(chunk([1, 2, 3, 4, 5], 2)).toEqual([[1, 2], [3, 4], [5]])
    expect(chunk([], 3)).toEqual([])
  })
})

describe('ancestorsOf', () => {
  const tree: Record<string, { id: string; name: string; parent_id: string | null }> = {
    a: { id: 'a', name: 'A', parent_id: null },
    b: { id: 'b', name: 'B', parent_id: 'a' },
    c: { id: 'c', name: 'C', parent_id: 'b' },
  }
  const fetchParent = (id: string) => Promise.resolve(tree[id])

  it('returns the path from the top down to the folder', async () => {
    await expect(ancestorsOf(tree.c, fetchParent)).resolves.toEqual([
      { id: 'a', name: 'A' },
      { id: 'b', name: 'B' },
      { id: 'c', name: 'C' },
    ])
  })
  it('stops on a cycle and at the depth limit', async () => {
    const loop = { id: 'x', name: 'X', parent_id: 'y' }
    const loopFetch = (id: string) => Promise.resolve(id === 'y' ? { id: 'y', name: 'Y', parent_id: 'x' } : loop)
    await expect(ancestorsOf(loop, loopFetch)).resolves.toHaveLength(2)
    await expect(ancestorsOf(tree.c, fetchParent, 2)).resolves.toHaveLength(2)
  })
})

describe('affectsFolder', () => {
  it('matches the event scope, and refreshes when the scope is unknown', () => {
    expect(affectsFolder({ parent_ids: ['f1', null] }, 'f1')).toBe(true)
    expect(affectsFolder({ parent_ids: ['f1', null] }, null)).toBe(true)
    expect(affectsFolder({ parent_ids: ['f1'] }, 'f2')).toBe(false)
    expect(affectsFolder({ parent_ids: [] }, 'f2')).toBe(true)
    expect(affectsFolder('garbage', 'f2')).toBe(true)
    expect(affectsFolder(undefined, null)).toBe(true)
  })
})
