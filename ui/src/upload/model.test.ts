import { describe, expect, it } from 'vitest'
import { directoriesOf } from './collect'
import { INITIAL_STATE, MAX_CONCURRENT, dirKey, durationParts, isActive, nextToStart, overall, remainingSeconds, toRows, uploadReducer, wasRenamed, type Transfer, type UploadAction, type UploadState } from './model'

let seq = 0
function transfer(change: Partial<Transfer> = {}): Transfer {
  seq += 1
  return { id: `t${seq}`, group: null, file: new File(['x'], 'a.txt'), name: 'a.txt', dirs: [], size: 100, parentId: null, destName: 'Files', status: 'waiting', loaded: 0, error: null, savedAs: null, ...change }
}
const run = (actions: UploadAction[], from: UploadState = INITIAL_STATE) => actions.reduce(uploadReducer, from)

describe('queue', () => {
  it('starts at most MAX_CONCURRENT transfers, in order', () => {
    const list = [transfer(), transfer(), transfer(), transfer()]
    expect(nextToStart(list)).toEqual([list[0].id, list[1].id])
    expect(MAX_CONCURRENT).toBe(2)
  })

  it('fills a freed slot when a transfer finishes', () => {
    const [a, b, c] = [transfer({ status: 'uploading' }), transfer({ status: 'uploading' }), transfer()]
    expect(nextToStart([a, b, c])).toEqual([])
    const after = run([{ type: 'done', id: a.id, savedAs: 'a.txt' }], { ...INITIAL_STATE, transfers: [a, b, c] })
    expect(nextToStart(after.transfers)).toEqual([c.id])
  })

  it('ignores finished, failed and cancelled transfers', () => {
    expect(nextToStart([transfer({ status: 'done' }), transfer({ status: 'failed' }), transfer({ status: 'cancelled' })])).toEqual([])
  })
})

describe('reducer', () => {
  it('records progress and completion with the name the server chose', () => {
    const t = transfer()
    const state = run([{ type: 'add', transfers: [t], now: 1 }, { type: 'start', id: t.id }, { type: 'progress', id: t.id, loaded: 40 }])
    expect(state.transfers[0]).toMatchObject({ status: 'uploading', loaded: 40 })
    const done = run([{ type: 'done', id: t.id, savedAs: 'a(1).txt' }], state)
    expect(done.transfers[0]).toMatchObject({ status: 'done', loaded: 100, savedAs: 'a(1).txt' })
    expect(isActive(done.transfers)).toBe(false)
  })

  it('cancel only affects unfinished transfers', () => {
    const [done, busy, wait] = [transfer({ status: 'done', loaded: 100 }), transfer({ status: 'uploading', loaded: 5 }), transfer()]
    const state = run([{ type: 'cancel', ids: [done.id, busy.id, wait.id] }], { ...INITIAL_STATE, transfers: [done, busy, wait] })
    expect(state.transfers.map((t) => t.status)).toEqual(['done', 'cancelled', 'cancelled'])
  })

  it('retry requeues failed and cancelled transfers from zero', () => {
    const failed = transfer({ status: 'failed', loaded: 30, error: 'error.NETWORK' })
    const state = run([{ type: 'retry', ids: [failed.id], now: 5 }], { ...INITIAL_STATE, transfers: [failed] })
    expect(state.transfers[0]).toMatchObject({ status: 'waiting', loaded: 0, error: null })
    expect(state.startedAt).toBe(5)
  })

  it('pause puts an interrupted transfer back in the queue', () => {
    const t = transfer({ status: 'uploading', loaded: 50 })
    expect(run([{ type: 'pause', id: t.id }], { ...INITIAL_STATE, transfers: [t] }).transfers[0]).toMatchObject({ status: 'waiting', loaded: 0 })
  })

  it('clear keeps only running work', () => {
    const keep = [transfer({ status: 'uploading' }), transfer()]
    const state = run([{ type: 'clear' }], { ...INITIAL_STATE, transfers: [...keep, transfer({ status: 'done' }), transfer({ status: 'failed' })] })
    expect(state.transfers).toEqual(keep)
  })

  it('keeps the original start time while work is still running', () => {
    const state = run([{ type: 'add', transfers: [transfer()], now: 10 }, { type: 'add', transfers: [transfer()], now: 99 }])
    expect(state.startedAt).toBe(10)
  })
})

describe('rows', () => {
  it('shows a dropped folder as one row with a file count and the server-chosen name', () => {
    const files = [
      transfer({ group: 'g1', dirs: ['Photos'], name: 'a.jpg', size: 10, status: 'done', loaded: 10 }),
      transfer({ group: 'g1', dirs: ['Photos', '2024'], name: 'b.jpg', size: 30, status: 'uploading', loaded: 15 }),
      transfer({ name: 'loose.txt' }),
    ]
    const rows = toRows({ ...INITIAL_STATE, transfers: files, groupNames: { g1: 'Photos(1)' } })
    expect(rows).toHaveLength(2)
    expect(rows[0]).toMatchObject({ kind: 'folder', name: 'Photos', savedAs: 'Photos(1)', count: 2, done: 1, loaded: 25, size: 40, status: 'uploading' })
    expect(rows[1]).toMatchObject({ kind: 'file', name: 'loose.txt', status: 'waiting' })
  })

  it('a folder with a failed file reports failure once nothing is running', () => {
    const rows = toRows({ ...INITIAL_STATE, transfers: [transfer({ group: 'g', dirs: ['D'], status: 'done' }), transfer({ group: 'g', dirs: ['D'], status: 'failed', error: 'error.NETWORK' })] })
    expect(rows[0]).toMatchObject({ status: 'failed', error: 'error.NETWORK' })
  })
})

describe('numbers and names', () => {
  it('overall ignores cancelled files and counts the unfinished', () => {
    const list = [transfer({ status: 'done', loaded: 100 }), transfer({ status: 'uploading', loaded: 20 }), transfer({ status: 'cancelled' }), transfer()]
    expect(overall(list)).toEqual({ loaded: 120, total: 300, remaining: 2 })
  })

  it('estimates the remaining time only with enough data', () => {
    expect(remainingSeconds(0, 100, 5000)).toBeNull()
    expect(remainingSeconds(10, 100, 500)).toBeNull()
    expect(remainingSeconds(100, 100, 5000)).toBeNull()
    expect(remainingSeconds(25, 100, 5000)).toBe(15)
  })

  it('rounds durations to one unit', () => {
    expect(durationParts(0.2)).toEqual({ value: 1, unit: 'second' })
    expect(durationParts(59)).toEqual({ value: 59, unit: 'second' })
    expect(durationParts(61)).toEqual({ value: 2, unit: 'minute' })
    expect(durationParts(7300)).toEqual({ value: 3, unit: 'hour' })
  })

  it('spots a rename by the server', () => {
    expect(wasRenamed('photo.jpg', 'photo(1).jpg')).toBe(true)
    expect(wasRenamed('photo.jpg', 'photo.jpg')).toBe(false)
    expect(wasRenamed('photo.jpg', null)).toBe(false)
  })

  it('derives folder paths from a relative path', () => {
    expect(directoriesOf('Photos/2024/a.jpg')).toEqual(['Photos', '2024'])
    expect(directoriesOf('a.jpg')).toEqual([])
    expect(directoriesOf('')).toEqual([])
    expect(dirKey(['Photos', '2024'])).toBe('Photos/2024')
  })
})
