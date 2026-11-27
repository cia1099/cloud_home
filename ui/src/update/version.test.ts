import { describe, expect, it } from 'vitest'
import { IDLE_RELOAD_MS, isStale, reloadDecision, safeNotesUrl, type ReloadInput } from './version'

const base: ReloadInput = { stale: true, busy: false, idleMs: 0, dismissed: false, alreadyTried: false }

describe('isStale', () => {
  it('differs only when the server reports another non-empty version', () => {
    expect(isStale('0.3.1', '0.3.1')).toBe(false)
    expect(isStale('0.3.1', '0.3.2')).toBe(true)
    expect(isStale('0.3.1-rc.1', '0.3.1')).toBe(true)
    expect(isStale('0.3.1', null)).toBe(false)
    expect(isStale('0.3.1', '')).toBe(false)
  })
})

describe('reloadDecision', () => {
  it('does nothing when the bundle is current', () => {
    expect(reloadDecision({ ...base, stale: false, idleMs: 10 * IDLE_RELOAD_MS })).toBe('none')
  })
  it('prompts while the user is active and reloads once idle', () => {
    expect(reloadDecision(base)).toBe('prompt')
    expect(reloadDecision({ ...base, idleMs: IDLE_RELOAD_MS - 1 })).toBe('prompt')
    expect(reloadDecision({ ...base, idleMs: IDLE_RELOAD_MS })).toBe('reload')
  })
  it('never reloads during an upload or download, however idle', () => {
    expect(reloadDecision({ ...base, busy: true })).toBe('wait')
    expect(reloadDecision({ ...base, busy: true, idleMs: 99 * IDLE_RELOAD_MS })).toBe('wait')
  })
  it('respects Later, which also stops the automatic reload', () => {
    expect(reloadDecision({ ...base, dismissed: true, idleMs: IDLE_RELOAD_MS })).toBe('none')
    expect(reloadDecision({ ...base, dismissed: true, busy: true })).toBe('none')
  })
  it('does not reload in a loop when a reload did not bring the new version', () => {
    expect(reloadDecision({ ...base, alreadyTried: true, idleMs: IDLE_RELOAD_MS })).toBe('prompt')
  })
})

describe('safeNotesUrl', () => {
  it('links only http(s) release pages', () => {
    expect(safeNotesUrl('https://github.com/o/r/releases/tag/v1')).toBe('https://github.com/o/r/releases/tag/v1')
    expect(safeNotesUrl('javascript:alert(1)')).toBeNull()
    expect(safeNotesUrl('not a url')).toBeNull()
  })
})
