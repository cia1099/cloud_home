import { describe, expect, it } from 'vitest'
import type { Usage } from './api/usage'
import { expiresSoon, expiryToIso, isExpired, shareUrl } from './shares/model'
import { isLow, percentOf, segments } from './storage/model'
import { daysLeft, isUrgent } from './trash/model'

const at = (y: number, m: number, d: number, h = 12) => new Date(y, m - 1, d, h).getTime()

describe('daysLeft', () => {
  it('counts calendar days in local time', () => {
    const now = at(2026, 11, 27, 15)
    expect(daysLeft(new Date(at(2026, 12, 4, 15)).toISOString(), now)).toBe(7)
    expect(daysLeft(new Date(at(2026, 11, 28, 1)).toISOString(), now)).toBe(1)
  })
  it('is 0 later today and never negative', () => {
    const now = at(2026, 11, 27, 9)
    expect(daysLeft(new Date(at(2026, 11, 27, 23)).toISOString(), now)).toBe(0)
    expect(daysLeft(new Date(at(2026, 11, 20)).toISOString(), now)).toBe(0)
    expect(daysLeft('not a date', now)).toBe(0)
  })
  it('flags one day or less as urgent', () => {
    expect([0, 1, 2].map(isUrgent)).toEqual([true, true, false])
  })
})

describe('share model', () => {
  const now = Date.UTC(2026, 10, 27, 12)
  it('turns the expiry choice into an RFC 3339 instant', () => {
    expect(expiryToIso('never', now)).toBeNull()
    expect(expiryToIso('1d', now)).toBe('2026-11-28T12:00:00.000Z')
    expect(expiryToIso('30d', now)).toBe('2026-12-27T12:00:00.000Z')
  })
  it('detects expired and soon-expiring shares', () => {
    expect(isExpired(null, now)).toBe(false)
    expect(isExpired('2026-11-27T11:59:59Z', now)).toBe(true)
    expect(isExpired('2026-11-27T12:00:01Z', now)).toBe(false)
    expect(expiresSoon('2026-11-30T12:00:00Z', now)).toBe(true)
    expect(expiresSoon('2026-12-01T12:00:00Z', now)).toBe(false)
    expect(expiresSoon(null, now)).toBe(false)
  })
  it('builds the public page address from the origin', () => {
    expect(shareUrl('https://home.example', 'a b/c')).toBe('https://home.example/s/a%20b%2Fc')
  })
})

describe('storage model', () => {
  const usage: Usage = { used_bytes: 100, trashed_bytes: 20, file_count: 3, volume_total_bytes: 1000, volume_available_bytes: 700, drive_available: true }
  it('puts the rest of the drive into "others"', () => {
    expect(segments(usage)).toEqual([
      { id: 'mine', bytes: 100 },
      { id: 'trash', bytes: 20 },
      { id: 'others', bytes: 180 },
      { id: 'free', bytes: 700 },
    ])
  })
  it('never reports negative space and handles an empty drive', () => {
    expect(segments({ ...usage, volume_available_bytes: 950 }).find((s) => s.id === 'others')?.bytes).toBe(0)
    expect(percentOf(5, 0)).toBe(0)
    expect(percentOf(250, 1000)).toBe(25)
  })
  it('warns under 10 % free only while the drive is connected', () => {
    expect(isLow({ ...usage, volume_available_bytes: 99 })).toBe(true)
    expect(isLow({ ...usage, volume_available_bytes: 100 })).toBe(false)
    expect(isLow({ ...usage, volume_available_bytes: 0, drive_available: false })).toBe(false)
  })
})
